use super::ProgressEvent;
use agent_client_protocol::{
    ContentBlock, ExtNotification, ExtRequest, ExtResponse, Meta, PermissionOptionKind,
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SelectedPermissionOutcome, SessionNotification, SessionUpdate, ToolKind,
};
use async_trait::async_trait;
use serde_json::value::RawValue;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::domain::ReviewTask;

/// Client implementation for receiving agent callbacks.
pub(super) struct LaReviewClient {
    pub(super) messages: Arc<Mutex<Vec<String>>>,
    pub(super) thoughts: Arc<Mutex<Vec<String>>>,
    pub(super) tasks: Arc<Mutex<Option<Vec<ReviewTask>>>>,
    pub(super) raw_tasks_payload: Arc<Mutex<Option<serde_json::Value>>>,
    last_message_id: Arc<Mutex<Option<String>>>,
    last_thought_id: Arc<Mutex<Option<String>>>,
    progress: Option<tokio::sync::mpsc::UnboundedSender<ProgressEvent>>,
    pr_id: String,
    has_repo_access: bool,
    repo_root: Option<PathBuf>,
}

impl LaReviewClient {
    fn parse_return_payload_from_str(payload: &str) -> Option<serde_json::Value> {
        serde_json::from_str::<serde_json::Value>(payload)
            .ok()
            .filter(|value| value.get("tasks").is_some() || value.get("plans").is_some())
    }

    fn looks_like_return_tool(
        &self,
        tool_title: &str,
        raw_input: &Option<serde_json::Value>,
    ) -> bool {
        if tool_title.contains("return_tasks") || tool_title.contains("return_plans") {
            return true;
        }

        if let Some(input) = raw_input
            && (input.get("tasks").is_some() || input.get("plans").is_some())
        {
            return true;
        }

        Self::parse_return_payload_from_str(tool_title).is_some()
    }

    pub(super) fn new(
        progress: Option<tokio::sync::mpsc::UnboundedSender<ProgressEvent>>,
        pr_id: impl Into<String>,
        repo_root: Option<PathBuf>,
    ) -> Self {
        let has_repo_access = repo_root.is_some();
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
            thoughts: Arc::new(Mutex::new(Vec::new())),
            tasks: Arc::new(Mutex::new(None)),
            raw_tasks_payload: Arc::new(Mutex::new(None)),
            last_message_id: Arc::new(Mutex::new(None)),
            last_thought_id: Arc::new(Mutex::new(None)),
            progress,
            pr_id: pr_id.into(),
            has_repo_access,
            repo_root,
        }
    }

    /// Attempt to parse and store tasks from arbitrary JSON value.
    fn store_tasks_from_value(&self, value: serde_json::Value) -> bool {
        let parsed = super::super::task_mcp_server::parse_tasks(value.clone());
        match parsed {
            Ok(mut tasks) => {
                for task in &mut tasks {
                    task.pr_id = self.pr_id.clone();
                }
                if let Ok(mut guard) = self.tasks.lock() {
                    *guard = Some(tasks);
                }
                if let Ok(mut guard) = self.raw_tasks_payload.lock() {
                    *guard = Some(value);
                }
                true
            }
            Err(err) => {
                eprintln!("[acp] failed to parse return_tasks payload: {err:?}");
                false
            }
        }
    }

    fn is_safe_read_request(&self, raw_input: &Option<serde_json::Value>) -> bool {
        let Some(root) = self.repo_root.as_ref() else {
            return false;
        };
        let Some(input) = raw_input.as_ref() else {
            return false;
        };
        let Some(path_str) = input.get("path").and_then(|v| v.as_str()) else {
            return false;
        };

        let requested = Path::new(path_str);
        let joined = if requested.is_absolute() {
            requested.to_path_buf()
        } else {
            root.join(requested)
        };

        let root_canon = root.canonicalize().unwrap_or_else(|_| root.clone());
        let joined_canon = joined.canonicalize().unwrap_or(joined);

        joined_canon.starts_with(&root_canon)
    }

    /// Handle task submission via extension payloads.
    fn handle_extension_payload(&self, method: &str, params: &RawValue) -> bool {
        if matches!(
            method,
            "lareview/return_tasks"
                | "return_tasks"
                | "lareview/create_review_tasks"
                | "create_review_tasks"
        ) && let Ok(value) = serde_json::from_str::<serde_json::Value>(params.get())
        {
            return self.store_tasks_from_value(value);
        }

        false
    }

    fn extract_chunk_id(meta: Option<&Meta>) -> Option<String> {
        meta.and_then(|meta| {
            ["message_id", "messageId", "id"]
                .iter()
                .find_map(|key| meta.get(*key).and_then(|val| val.as_str()))
                .map(|s| s.to_string())
        })
    }

    fn append_streamed_content(
        &self,
        store: &Arc<Mutex<Vec<String>>>,
        last_id: &Arc<Mutex<Option<String>>>,
        meta: Option<&Meta>,
        text: &str,
    ) -> (String, bool) {
        let chunk_id = Self::extract_chunk_id(meta);
        let mut id_guard = last_id.lock().unwrap();
        let mut store_guard = store.lock().unwrap();

        let mut is_new = false;

        if let Some(ref incoming) = chunk_id {
            if id_guard.as_deref() != Some(incoming.as_str()) {
                store_guard.push(String::new());
                *id_guard = Some(incoming.clone());
                is_new = true;
            }
        } else if store_guard.is_empty() {
            store_guard.push(String::new());
            is_new = true;
        }

        if store_guard.is_empty() {
            store_guard.push(String::new());
            is_new = true;
        }

        if id_guard.is_none() {
            *id_guard = chunk_id;
        }

        if let Some(last) = store_guard.last_mut() {
            last.push_str(text);
        }

        let combined = store_guard.last().cloned().unwrap_or_default();
        (combined, is_new)
    }
}

#[async_trait(?Send)]
impl agent_client_protocol::Client for LaReviewClient {
    async fn request_permission(
        &self,
        args: RequestPermissionRequest,
    ) -> agent_client_protocol::Result<RequestPermissionResponse> {
        let tool_kind = args.tool_call.fields.kind;
        let tool_title = args.tool_call.fields.title.clone().unwrap_or_default();

        let raw_input = &args.tool_call.fields.raw_input;
        let is_return_tool = self.looks_like_return_tool(&tool_title, raw_input);
        let allow_option = if is_return_tool {
            args.options.iter().find(|opt| {
                matches!(
                    opt.kind,
                    PermissionOptionKind::AllowOnce | PermissionOptionKind::AllowAlways
                )
            })
        } else if self.has_repo_access
            && matches!(tool_kind, Some(ToolKind::Read))
            && self.is_safe_read_request(raw_input)
        {
            args.options.iter().find(|opt| {
                matches!(
                    opt.kind,
                    PermissionOptionKind::AllowOnce | PermissionOptionKind::AllowAlways
                )
            })
        } else {
            None
        };

        let outcome = allow_option
            .map(|option| {
                RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(
                    option.option_id.clone(),
                ))
            })
            .unwrap_or(RequestPermissionOutcome::Cancelled);

        eprintln!(
            "[acp] permission request: kind={tool_kind:?} title={tool_title:?} outcome={outcome:?}"
        );
        Ok(RequestPermissionResponse::new(outcome))
    }

    async fn session_notification(
        &self,
        notification: SessionNotification,
    ) -> agent_client_protocol::Result<()> {
        // Debug log all updates when ACP_DEBUG is set
        if std::env::var("ACP_DEBUG").is_ok() {
            eprintln!("[acp] session update: {:?}", notification.update);
        }

        let update = notification.update.clone();

        match &update {
            SessionUpdate::AgentMessageChunk(chunk) => {
                if let ContentBlock::Text(text) = &chunk.content {
                    let _ = self.append_streamed_content(
                        &self.messages,
                        &self.last_message_id,
                        chunk.meta.as_ref(),
                        &text.text,
                    );
                }
            }
            SessionUpdate::AgentThoughtChunk(chunk) => {
                if let ContentBlock::Text(text) = &chunk.content {
                    let _ = self.append_streamed_content(
                        &self.thoughts,
                        &self.last_thought_id,
                        chunk.meta.as_ref(),
                        &text.text,
                    );
                }
            }
            SessionUpdate::ToolCall(call) => {
                // Debug log tool call details
                if std::env::var("ACP_DEBUG").is_ok() {
                    eprintln!(
                        "[acp] tool call: title={:?}, raw_input={:?}, raw_output={:?}",
                        call.title, call.raw_input, call.raw_output
                    );
                }

                let is_task_tool = self.looks_like_return_tool(&call.title, &call.raw_input)
                    || call.title.contains("create_review_tasks");

                if is_task_tool {
                    if let Some(ref input) = call.raw_input {
                        self.store_tasks_from_value(input.clone());
                    }
                    if let Some(ref output) = call.raw_output {
                        self.store_tasks_from_value(output.clone());
                    }
                    if call.raw_input.is_none()
                        && call.raw_output.is_none()
                        && let Some(value) = Self::parse_return_payload_from_str(&call.title)
                    {
                        self.store_tasks_from_value(value);
                    }
                }
            }
            _ => {}
        }

        if let Some(tx) = &self.progress {
            let _ = tx.send(ProgressEvent::Update(Box::new(update)));
        }
        Ok(())
    }

    async fn ext_method(&self, args: ExtRequest) -> agent_client_protocol::Result<ExtResponse> {
        let stored = self.handle_extension_payload(&args.method, &args.params);
        let response_value = if stored {
            serde_json::json!({ "status": "ok" })
        } else {
            serde_json::json!({ "status": "ignored" })
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
