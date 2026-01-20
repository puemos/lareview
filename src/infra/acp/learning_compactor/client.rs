use agent_client_protocol::{
    ContentBlock, Error, ExtNotification, ExtRequest, ExtResponse, PermissionOptionKind,
    ReadTextFileRequest, ReadTextFileResponse, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, SelectedPermissionOutcome, SessionNotification, SessionUpdate,
};
use async_trait::async_trait;
use log::debug;
use serde_json::value::RawValue;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

use crate::domain::LearnedPatternInput;

const TOOL_SUBMIT_LEARNED_PATTERNS: &str = "submit_learned_patterns";
const TOOL_FINALIZE_LEARNING: &str = "finalize_learning";
const LEARNING_TOOLS: &[&str] = &[TOOL_SUBMIT_LEARNED_PATTERNS, TOOL_FINALIZE_LEARNING];

pub struct LearningClient {
    pub patterns: Arc<Mutex<Vec<LearnedPatternInput>>>,
    pub finalization_received: Arc<Mutex<bool>>,
    pub messages: Arc<Mutex<Vec<String>>>,
    pub thoughts: Arc<Mutex<Vec<String>>>,
    progress_tx: Option<tokio::sync::mpsc::UnboundedSender<LearningProgressEvent>>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum LearningProgressEvent {
    LocalLog(String),
    PatternSubmitted { pattern_text: String },
    Finalized,
}

impl LearningClient {
    pub fn new(
        progress_tx: Option<tokio::sync::mpsc::UnboundedSender<LearningProgressEvent>>,
    ) -> Self {
        Self {
            patterns: Arc::new(Mutex::new(Vec::new())),
            finalization_received: Arc::new(Mutex::new(false)),
            messages: Arc::new(Mutex::new(Vec::new())),
            thoughts: Arc::new(Mutex::new(Vec::new())),
            progress_tx,
        }
    }

    fn looks_like_learning_tool(&self, tool_title: &str) -> bool {
        LEARNING_TOOLS.iter().any(|tool| tool_title.contains(tool))
    }

    fn tool_name_from_title(title: &str) -> Option<&'static str> {
        LEARNING_TOOLS
            .iter()
            .find(|tool| title.contains(*tool))
            .copied()
    }

    fn tool_name_from_payload(payload: &Value) -> Option<String> {
        let parsed = if let Some(s) = payload.as_str() {
            serde_json::from_str::<Value>(s).ok()
        } else {
            Some(payload.clone())
        }?;

        parsed
            .get("tool")
            .and_then(|v| v.as_str())
            .or_else(|| parsed.get("name").and_then(|v| v.as_str()))
            .map(|s| s.to_string())
    }

    fn mark_finalization_received(&self) {
        if let Ok(mut guard) = self.finalization_received.lock()
            && !*guard
        {
            *guard = true;
            if let Some(tx) = &self.progress_tx {
                let _ = tx.send(LearningProgressEvent::Finalized);
            }
        }
    }

    fn emit_log(&self, message: impl Into<String>) {
        if let Some(tx) = &self.progress_tx {
            let _ = tx.send(LearningProgressEvent::LocalLog(message.into()));
        }
    }

    fn process_patterns_input(&self, input: &Value) {
        if let Some(patterns_array) = input.get("patterns").and_then(|v| v.as_array()) {
            for pattern_value in patterns_array {
                let pattern_text = match pattern_value.get("pattern_text").and_then(|v| v.as_str())
                {
                    Some(text) => text.to_string(),
                    None => continue,
                };

                let category = pattern_value
                    .get("category")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let file_extension = pattern_value
                    .get("file_extension")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let pattern_input = LearnedPatternInput {
                    pattern_text: pattern_text.clone(),
                    category,
                    file_extension,
                    enabled: Some(true),
                };

                if let Ok(mut guard) = self.patterns.lock() {
                    guard.push(pattern_input);
                }

                if let Some(tx) = &self.progress_tx {
                    let _ = tx.send(LearningProgressEvent::PatternSubmitted { pattern_text });
                }
            }
        }
    }

    fn handle_extension_payload(&self, method: &str, params: &RawValue) -> bool {
        if matches!(
            method,
            "lareview/submit_learned_patterns" | "submit_learned_patterns"
        ) && let Ok(value) = serde_json::from_str::<Value>(params.get())
        {
            self.process_patterns_input(&value);
            return true;
        }

        if matches!(method, "lareview/finalize_learning" | "finalize_learning") {
            self.mark_finalization_received();
            return true;
        }

        false
    }
}

#[async_trait(?Send)]
impl agent_client_protocol::Client for LearningClient {
    async fn request_permission(
        &self,
        args: RequestPermissionRequest,
    ) -> agent_client_protocol::Result<RequestPermissionResponse> {
        debug!(
            target: "acp",
            "learning: request_permission called: tool_kind={:?}, title={:?}",
            args.tool_call.fields.kind,
            args.tool_call.fields.title
        );

        let tool_title = args.tool_call.fields.title.clone().unwrap_or_default();
        let raw_input = &args.tool_call.fields.raw_input;
        let tool_name = raw_input.as_ref().and_then(Self::tool_name_from_payload);

        let is_learning_tool = self.looks_like_learning_tool(&tool_title)
            || tool_name
                .as_deref()
                .map(|name| LEARNING_TOOLS.contains(&name))
                .unwrap_or(false);

        let allow_option = if is_learning_tool {
            self.emit_log(format!("learning: allow {}", tool_title));
            args.options.iter().find(|opt| {
                matches!(
                    opt.kind,
                    PermissionOptionKind::AllowOnce | PermissionOptionKind::AllowAlways
                )
            })
        } else {
            self.emit_log(format!(
                "learning: deny {} (not a learning tool)",
                tool_title
            ));
            None
        };

        let outcome = allow_option
            .map(|option| {
                RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(
                    option.option_id.clone(),
                ))
            })
            .unwrap_or(RequestPermissionOutcome::Cancelled);

        Ok(RequestPermissionResponse::new(outcome))
    }

    async fn read_text_file(
        &self,
        _args: ReadTextFileRequest,
    ) -> agent_client_protocol::Result<ReadTextFileResponse> {
        Err(Error::invalid_params().data(json!({
            "reason": "file access not available in learning mode"
        })))
    }

    async fn session_notification(
        &self,
        notification: SessionNotification,
    ) -> agent_client_protocol::Result<()> {
        debug!(target: "acp", "learning: session update: {:?}", notification.update);

        match &notification.update {
            SessionUpdate::AgentMessageChunk(chunk) => {
                if let ContentBlock::Text(text) = &chunk.content
                    && let Ok(mut guard) = self.messages.lock()
                {
                    if guard.is_empty() {
                        guard.push(String::new());
                    }
                    if let Some(last) = guard.last_mut() {
                        last.push_str(&text.text);
                    }
                }
            }
            SessionUpdate::AgentThoughtChunk(chunk) => {
                if let ContentBlock::Text(text) = &chunk.content
                    && let Ok(mut guard) = self.thoughts.lock()
                {
                    if guard.is_empty() {
                        guard.push(String::new());
                    }
                    if let Some(last) = guard.last_mut() {
                        last.push_str(&text.text);
                    }
                }
            }
            SessionUpdate::ToolCall(call) => {
                debug!(
                    target: "acp",
                    "learning: tool call: title={:?}, raw_input={:?}",
                    call.title, call.raw_input
                );

                let tool_name = Self::tool_name_from_title(&call.title)
                    .map(|s| s.to_string())
                    .or_else(|| {
                        call.raw_input
                            .as_ref()
                            .and_then(Self::tool_name_from_payload)
                    });

                let is_submit_patterns =
                    matches!(tool_name.as_deref(), Some(TOOL_SUBMIT_LEARNED_PATTERNS))
                        || call.title.contains(TOOL_SUBMIT_LEARNED_PATTERNS);
                let is_finalize = matches!(tool_name.as_deref(), Some(TOOL_FINALIZE_LEARNING))
                    || call.title.contains(TOOL_FINALIZE_LEARNING);

                if is_submit_patterns {
                    if let Some(ref input) = call.raw_input {
                        self.process_patterns_input(input);
                    }
                } else if is_finalize {
                    self.mark_finalization_received();
                }
            }
            SessionUpdate::ToolCallUpdate(update) => {
                let tool_name = update
                    .fields
                    .title
                    .as_deref()
                    .and_then(Self::tool_name_from_title)
                    .map(|s| s.to_string())
                    .or_else(|| {
                        update
                            .fields
                            .raw_input
                            .as_ref()
                            .and_then(Self::tool_name_from_payload)
                    });

                let is_completed = matches!(
                    update.fields.status,
                    Some(agent_client_protocol::ToolCallStatus::Completed)
                );
                let is_finalize = matches!(tool_name.as_deref(), Some(TOOL_FINALIZE_LEARNING));
                let is_submit_patterns =
                    matches!(tool_name.as_deref(), Some(TOOL_SUBMIT_LEARNED_PATTERNS));

                if is_completed {
                    if is_finalize {
                        debug!(target: "acp", "learning: finalize_learning completed");
                        self.mark_finalization_received();
                    } else if is_submit_patterns && let Some(ref input) = update.fields.raw_input {
                        self.process_patterns_input(input);
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    async fn ext_method(&self, args: ExtRequest) -> agent_client_protocol::Result<ExtResponse> {
        let stored = self.handle_extension_payload(&args.method, &args.params);
        let response_value = if stored {
            json!({ "status": "ok" })
        } else {
            json!({ "status": "ignored" })
        };
        let raw = RawValue::from_string(response_value.to_string())
            .map(Arc::from)
            .unwrap_or_else(|_| Arc::from(RawValue::from_string("null".into()).unwrap()));
        Ok(ExtResponse::new(raw))
    }

    async fn ext_notification(&self, args: ExtNotification) -> agent_client_protocol::Result<()> {
        self.handle_extension_payload(&args.method, &args.params);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_looks_like_learning_tool() {
        let client = LearningClient::new(None);
        assert!(client.looks_like_learning_tool("submit_learned_patterns"));
        assert!(client.looks_like_learning_tool("finalize_learning"));
        assert!(!client.looks_like_learning_tool("read_file"));
    }

    #[test]
    fn test_tool_name_from_title() {
        assert_eq!(
            LearningClient::tool_name_from_title("submit_learned_patterns"),
            Some("submit_learned_patterns")
        );
        assert_eq!(
            LearningClient::tool_name_from_title("finalize_learning"),
            Some("finalize_learning")
        );
        assert_eq!(LearningClient::tool_name_from_title("other_tool"), None);
    }

    #[test]
    fn test_process_patterns_input() {
        let client = LearningClient::new(None);
        let input = json!({
            "patterns": [
                {
                    "pattern_text": "Don't flag unwrap() in test files",
                    "category": "testing",
                    "file_extension": "rs",
                    "source_count": 3
                }
            ]
        });

        client.process_patterns_input(&input);

        let patterns = client.patterns.lock().unwrap();
        assert_eq!(patterns.len(), 1);
        assert_eq!(
            patterns[0].pattern_text,
            "Don't flag unwrap() in test files"
        );
        assert_eq!(patterns[0].category, Some("testing".to_string()));
        assert_eq!(patterns[0].file_extension, Some("rs".to_string()));
    }

    #[test]
    fn test_mark_finalization_received() {
        let client = LearningClient::new(None);
        assert!(!*client.finalization_received.lock().unwrap());

        client.mark_finalization_received();
        assert!(*client.finalization_received.lock().unwrap());
    }
}
