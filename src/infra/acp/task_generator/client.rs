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
    pub(super) tasks: Arc<Mutex<Vec<ReviewTask>>>, // Changed to accumulate tasks
    pub(super) finalization_received: Arc<Mutex<bool>>, // Track if finalize_review was called
    pub(super) raw_tasks_payload: Arc<Mutex<Option<serde_json::Value>>>,
    last_message_id: Arc<Mutex<Option<String>>>,
    last_thought_id: Arc<Mutex<Option<String>>>,
    progress: Option<tokio::sync::mpsc::UnboundedSender<ProgressEvent>>,
    run_id: String,
    has_repo_access: bool,
    repo_root: Option<PathBuf>,
}

impl LaReviewClient {
    fn parse_return_payload_from_str(payload: &str) -> Option<serde_json::Value> {
        serde_json::from_str::<serde_json::Value>(payload)
            .ok()
            .filter(|value| {
                // Accept single task payload if it has id and diff_refs (and likely title)
                (value.get("id").is_some() && value.get("diff_refs").is_some()) ||
                // Accept finalize payload if it has title
                value.get("title").is_some() ||
                // Keep supporting plans for compatibility
                value.get("plans").is_some() ||
                // For backward compatibility, still accept bulk tasks
                value.get("tasks").is_some()
            })
    }

    fn looks_like_return_tool(&self, tool_title: &str) -> bool {
        tool_title.contains("return_task")
            || tool_title.contains("return_plans")
            || tool_title.contains("finalize_review")
    }

    pub(super) fn new(
        progress: Option<tokio::sync::mpsc::UnboundedSender<ProgressEvent>>,
        run_id: impl Into<String>,
        repo_root: Option<PathBuf>,
    ) -> Self {
        let has_repo_access = repo_root.is_some();
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
            thoughts: Arc::new(Mutex::new(Vec::new())),
            tasks: Arc::new(Mutex::new(Vec::new())), // Changed to accumulate tasks
            finalization_received: Arc::new(Mutex::new(false)), // Track if finalize_review was called
            raw_tasks_payload: Arc::new(Mutex::new(None)),
            last_message_id: Arc::new(Mutex::new(None)),
            last_thought_id: Arc::new(Mutex::new(None)),
            progress,
            run_id: run_id.into(),
            has_repo_access,
            repo_root,
        }
    }

    /// Attempt to append a single task from JSON value (for return_task).
    fn append_single_task_from_value(&self, value: serde_json::Value) -> bool {
        let parsed = super::super::task_mcp_server::parse_task(value.clone());
        match parsed {
            Ok(mut task) => {
                task.run_id = self.run_id.clone();
                if let Ok(mut guard) = self.tasks.lock() {
                    guard.push(task);
                }
                // Don't store raw payload for streaming - validation now works from task objects
                true
            }
            Err(err) => {
                eprintln!("[acp] failed to parse return_task payload: {err:?}");
                false
            }
        }
    }

    /// Mark finalization as received.
    fn mark_finalization_received(&self) {
        if let Ok(mut guard) = self.finalization_received.lock()
            && !*guard
        {
            *guard = true;
            if let Some(tx) = &self.progress {
                let _ = tx.send(ProgressEvent::Finalized);
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

        // If canonicalization fails, we cannot safely determine if the path is within the root,
        // so we err on the side of caution and return false
        let Ok(root_canon) = root.canonicalize() else {
            return false;
        };
        let Ok(joined_canon) = joined.canonicalize() else {
            return false;
        };

        joined_canon.starts_with(&root_canon)
    }

    /// Handle task submission via extension payloads.
    fn handle_extension_payload(&self, method: &str, params: &RawValue) -> bool {
        if matches!(method, "lareview/return_task" | "return_task")
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(params.get())
        {
            return self.append_single_task_from_value(value);
        }

        if matches!(method, "lareview/finalize_review" | "finalize_review")
            && let Ok(_value) = serde_json::from_str::<serde_json::Value>(params.get())
        {
            // For finalize_review, we don't need to do anything special, just note that it was received
            self.mark_finalization_received();
            return true;
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

        // Check if this looks like a return tool by checking tool name or if it's JSON in title that looks like a task/finalize payload
        let is_return_tool = self.looks_like_return_tool(&tool_title) || {
            // For ToolKind::Other, check if the title is JSON with required streaming fields
            matches!(tool_kind, Some(ToolKind::Other))
                && Self::parse_return_payload_from_str(&tool_title)
                    .map(|value| {
                        // Check if it's a single task (has id and diff_refs) or finalize review (has title)
                        (value.get("id").is_some() && value.get("diff_refs").is_some())
                            || value.get("title").is_some()
                    })
                    .unwrap_or(false)
        };

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
                // Important: No progress update for messages because they're not the final result
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
                // Important: No progress update for thoughts because they're not the final result
            }
            SessionUpdate::ToolCall(call) => {
                // Debug log tool call details
                if std::env::var("ACP_DEBUG").is_ok() {
                    eprintln!(
                        "[acp] tool call: title={:?}, raw_input={:?}, raw_output={:?}",
                        call.title, call.raw_input, call.raw_output
                    );
                }

                let is_return_task_tool = call.title.contains("return_task");
                let is_finalize_tool = call.title.contains("finalize_review");

                if is_return_task_tool {
                    // Handle streaming return_task
                    if let Some(ref input) = call.raw_input {
                        self.append_single_task_from_value(input.clone());
                    }
                    if let Some(ref output) = call.raw_output {
                        self.append_single_task_from_value(output.clone());
                    }
                    if call.raw_input.is_none()
                        && call.raw_output.is_none()
                        && let Some(value) = Self::parse_return_payload_from_str(&call.title)
                    {
                        self.append_single_task_from_value(value);
                    }
                } else if is_finalize_tool {
                    // Handle finalize_review
                    if let Some(ref input) = call.raw_input
                        && input.get("title").is_some()
                    {
                        self.mark_finalization_received();
                    }
                    if let Some(ref output) = call.raw_output
                        && output.get("title").is_some()
                    {
                        self.mark_finalization_received();
                    }
                    if call.raw_input.is_none()
                        && call.raw_output.is_none()
                        && let Some(value) = Self::parse_return_payload_from_str(&call.title)
                        && value.get("title").is_some()
                    {
                        self.mark_finalization_received();
                    }
                }
            }
            SessionUpdate::ToolCallUpdate(update) => {
                // Check if this is a finalize_review tool completing
                let tool_id: &str = &update.tool_call_id.0;
                let is_finalize = tool_id.contains("finalize_review");
                let is_completed = matches!(
                    update.fields.status,
                    Some(agent_client_protocol::ToolCallStatus::Completed)
                );

                if is_finalize && is_completed {
                    if std::env::var("ACP_DEBUG").is_ok() {
                        eprintln!("[acp] finalize_review completed via ToolCallUpdate");
                    }
                    self.mark_finalization_received();
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
