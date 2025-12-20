use super::ProgressEvent;
use agent_client_protocol::{
    ContentBlock, Error, ExtNotification, ExtRequest, ExtResponse, Meta, PermissionOptionKind,
    ReadTextFileRequest, ReadTextFileResponse, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, SelectedPermissionOutcome, SessionNotification, SessionUpdate,
    ToolKind,
};
use async_trait::async_trait;
use serde_json::json;
use serde_json::value::RawValue;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::domain::ReviewTask;

struct ReadCheck {
    allowed: bool,
    path_display: String,
    reason: String,
}

impl ReadCheck {
    fn allow(path_display: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            allowed: true,
            path_display: path_display.into(),
            reason: reason.into(),
        }
    }

    fn deny(path_display: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            path_display: path_display.into(),
            reason: reason.into(),
        }
    }
}

/// Client implementation for receiving agent callbacks.
pub(super) struct LaReviewClient {
    pub(super) messages: Arc<Mutex<Vec<String>>>,
    pub(super) thoughts: Arc<Mutex<Vec<String>>>,
    pub(super) tasks: Arc<Mutex<Vec<ReviewTask>>>, // Changed to accumulate tasks
    pub(super) finalization_received: Arc<Mutex<bool>>, // Track if finalize_review was called
    pub(super) raw_tasks_payload: Arc<Mutex<Option<serde_json::Value>>>,
    tool_call_names: Arc<Mutex<HashMap<String, String>>>,
    last_message_id: Arc<Mutex<Option<String>>>,
    last_thought_id: Arc<Mutex<Option<String>>>,
    progress: Option<tokio::sync::mpsc::UnboundedSender<ProgressEvent>>,
    run_id: String,
    has_repo_access: bool,
    repo_root: Option<PathBuf>,
}

impl LaReviewClient {
    fn parse_return_payload_from_str(payload: &str) -> Option<serde_json::Value> {
        let parsed = serde_json::from_str::<serde_json::Value>(payload).ok()?;
        for candidate in Self::payload_candidates(&parsed) {
            // Accept single task payload if it has id and diff_refs (and likely title)
            if candidate.get("id").is_some() && candidate.get("diff_refs").is_some() {
                return Some(candidate);
            }
            // Accept finalize payload if it has title
            if candidate.get("title").is_some() {
                return Some(candidate);
            }
            // Keep supporting plans for compatibility
            if candidate.get("plans").is_some() {
                return Some(candidate);
            }
            // For backward compatibility, still accept bulk tasks
            if candidate.get("tasks").is_some() {
                return Some(candidate);
            }
        }
        None
    }

    fn looks_like_return_tool(&self, tool_title: &str) -> bool {
        tool_title.contains("return_task")
            || tool_title.contains("return_plans")
            || tool_title.contains("finalize_review")
    }

    fn tool_name_from_title(title: &str) -> Option<&'static str> {
        if title.contains("return_task") {
            Some("return_task")
        } else if title.contains("return_plans") {
            Some("return_plans")
        } else if title.contains("finalize_review") {
            Some("finalize_review")
        } else {
            None
        }
    }

    fn tool_name_from_payload(payload: &serde_json::Value) -> Option<String> {
        let parsed = if let Some(s) = payload.as_str() {
            serde_json::from_str::<serde_json::Value>(s).ok()
        } else {
            Some(payload.clone())
        }?;

        parsed
            .get("tool")
            .and_then(|value| value.as_str())
            .or_else(|| parsed.get("name").and_then(|value| value.as_str()))
            .map(|value| value.to_string())
    }

    fn payload_candidates(value: &serde_json::Value) -> Vec<serde_json::Value> {
        let mut candidates = Vec::new();
        if let Some(s) = value.as_str()
            && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s)
        {
            candidates.push(parsed.clone());
            if let Some(nested) = Self::extract_nested_payload(&parsed) {
                candidates.push(nested);
            }
            return candidates;
        }

        candidates.push(value.clone());
        if let Some(nested) = Self::extract_nested_payload(value) {
            candidates.push(nested);
        }
        candidates
    }

    fn extract_nested_payload(value: &serde_json::Value) -> Option<serde_json::Value> {
        let candidate = value
            .get("arguments")
            .or_else(|| value.get("params"))
            .or_else(|| value.get("input"))
            .or_else(|| value.get("args"))
            .or_else(|| value.get("data"));

        match candidate {
            Some(inner) => {
                if let Some(s) = inner.as_str() {
                    serde_json::from_str::<serde_json::Value>(s).ok()
                } else {
                    Some(inner.clone())
                }
            }
            None => None,
        }
    }

    fn payload_has_title(value: &serde_json::Value) -> bool {
        Self::payload_candidates(value).iter().any(|candidate| {
            candidate
                .get("title")
                .and_then(|value| value.as_str())
                .is_some()
        })
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
            tool_call_names: Arc::new(Mutex::new(HashMap::new())),
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
        for candidate in Self::payload_candidates(&value) {
            let parsed = super::super::task_mcp_server::parse_task(candidate);
            if let Ok(mut task) = parsed {
                task.run_id = self.run_id.clone();
                if let Ok(mut guard) = self.tasks.lock() {
                    guard.push(task);
                }
                // Don't store raw payload for streaming - validation now works from task objects
                return true;
            }
        }
        false
    }

    fn record_tool_call_name(
        &self,
        tool_call_id: &agent_client_protocol::ToolCallId,
        tool_name: &str,
    ) {
        if let Ok(mut guard) = self.tool_call_names.lock() {
            guard.insert(tool_call_id.to_string(), tool_name.to_string());
        }
    }

    fn lookup_tool_call_name(
        &self,
        tool_call_id: &agent_client_protocol::ToolCallId,
    ) -> Option<String> {
        let key = tool_call_id.to_string();
        self.tool_call_names
            .lock()
            .ok()
            .and_then(|guard| guard.get(key.as_str()).cloned())
    }

    fn clear_tool_call_name(&self, tool_call_id: &agent_client_protocol::ToolCallId) {
        if let Ok(mut guard) = self.tool_call_names.lock() {
            let key = tool_call_id.to_string();
            guard.remove(key.as_str());
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

    fn emit_log(&self, message: impl Into<String>) {
        if let Some(tx) = &self.progress {
            let _ = tx.send(ProgressEvent::LocalLog(message.into()));
        }
    }

    fn normalize_path(path: &Path) -> PathBuf {
        let mut normalized = PathBuf::new();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    let _ = normalized.pop();
                }
                _ => normalized.push(component.as_os_str()),
            }
        }
        normalized
    }

    fn slice_lines(content: &str, line: Option<u32>, limit: Option<u32>) -> String {
        if line.is_none() && limit.is_none() {
            return content.to_string();
        }

        let start = line.unwrap_or(1).saturating_sub(1) as usize;
        let max = limit.unwrap_or(u32::MAX) as usize;
        content
            .lines()
            .skip(start)
            .take(max)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn resolve_repo_path(&self, requested: &Path) -> Result<PathBuf, Error> {
        let Some(root) = self.repo_root.as_ref() else {
            return Err(Error::invalid_params().data(json!({
                "reason": "repo access disabled",
                "path": requested.display().to_string(),
            })));
        };

        let root_canon = root.canonicalize().map_err(|err| {
            Error::resource_not_found(Some(root.display().to_string()))
                .data(json!({ "reason": err.to_string() }))
        })?;

        let joined = if requested.is_absolute() {
            requested.to_path_buf()
        } else {
            root_canon.join(requested)
        };

        let normalized = Self::normalize_path(&joined);
        if !normalized.starts_with(&root_canon) {
            return Err(Error::invalid_params().data(json!({
                "reason": "path outside repo root",
                "path": requested.display().to_string(),
            })));
        }

        let resolved = match joined.canonicalize() {
            Ok(path) => path,
            Err(_) => normalized,
        };

        if !resolved.starts_with(&root_canon) {
            return Err(Error::invalid_params().data(json!({
                "reason": "path resolves outside repo root",
                "path": requested.display().to_string(),
            })));
        }

        Ok(resolved)
    }

    fn check_read_request(&self, raw_input: &Option<serde_json::Value>) -> ReadCheck {
        let Some(root) = self.repo_root.as_ref() else {
            return ReadCheck::deny("<none>", "repo access disabled");
        };
        let Some(input) = raw_input.as_ref() else {
            return ReadCheck::deny("<missing>", "missing tool input");
        };
        let Some(path_str) = input.get("path").and_then(|v| v.as_str()) else {
            return ReadCheck::deny("<missing>", "missing `path` in tool input");
        };

        let root_canon = match root.canonicalize() {
            Ok(path) => path,
            Err(err) => {
                return ReadCheck::deny(
                    path_str,
                    format!("repo root not accessible: {} ({err})", root.display()),
                );
            }
        };

        let requested = Path::new(path_str);
        let joined = if requested.is_absolute() {
            requested.to_path_buf()
        } else {
            root_canon.join(requested)
        };

        let normalized = Self::normalize_path(&joined);
        if !normalized.starts_with(&root_canon) {
            return ReadCheck::deny(
                path_str,
                format!("path outside repo root: {}", normalized.display()),
            );
        }

        if let Ok(resolved) = joined.canonicalize() {
            if !resolved.starts_with(&root_canon) {
                return ReadCheck::deny(
                    path_str,
                    format!("path resolves outside repo root: {}", resolved.display()),
                );
            }
            return ReadCheck::allow(path_str, format!("read allowed: {}", resolved.display()));
        }

        let note = if joined.exists() {
            "non-canonical path"
        } else {
            "path not found"
        };
        ReadCheck::allow(
            path_str,
            format!("read allowed ({note}): {}", normalized.display()),
        )
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
        let tool_name = raw_input.as_ref().and_then(Self::tool_name_from_payload);

        // Check if this looks like a return tool by checking tool name or if it's JSON in title that looks like a task/finalize payload
        let is_return_tool = self.looks_like_return_tool(&tool_title)
            || matches!(
                tool_name.as_deref(),
                Some("return_task") | Some("return_plans") | Some("finalize_review")
            )
            || {
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

        let is_repo_tool =
            tool_title.contains("repo_search") || tool_title.contains("repo_list_files");

        let allow_option = if is_return_tool {
            args.options.iter().find(|opt| {
                matches!(
                    opt.kind,
                    PermissionOptionKind::AllowOnce | PermissionOptionKind::AllowAlways
                )
            })
        } else if is_repo_tool {
            let allowed = self.has_repo_access;
            let tool_label = if tool_title.is_empty() {
                "repo_tool"
            } else {
                tool_title.as_str()
            };
            self.emit_log(format!(
                "{}: {} ({})",
                tool_label,
                if allowed { "allow" } else { "deny" },
                if allowed {
                    "repo access enabled"
                } else {
                    "repo access disabled"
                }
            ));
            if allowed {
                args.options.iter().find(|opt| {
                    matches!(
                        opt.kind,
                        PermissionOptionKind::AllowOnce | PermissionOptionKind::AllowAlways
                    )
                })
            } else {
                None
            }
        } else if matches!(tool_kind, Some(ToolKind::Read)) {
            let check = self.check_read_request(raw_input);
            let tool_label = if tool_title.is_empty() {
                "fs/read_text_file"
            } else {
                tool_title.as_str()
            };
            self.emit_log(format!(
                "repo access: {} {} path={} ({})",
                if check.allowed { "allow" } else { "deny" },
                tool_label,
                check.path_display,
                check.reason
            ));

            if check.allowed {
                args.options.iter().find(|opt| {
                    matches!(
                        opt.kind,
                        PermissionOptionKind::AllowOnce | PermissionOptionKind::AllowAlways
                    )
                })
            } else {
                None
            }
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

        Ok(RequestPermissionResponse::new(outcome))
    }

    async fn read_text_file(
        &self,
        args: ReadTextFileRequest,
    ) -> agent_client_protocol::Result<ReadTextFileResponse> {
        let resolved = self.resolve_repo_path(&args.path)?;
        let content = std::fs::read_to_string(&resolved).map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                Error::resource_not_found(Some(resolved.display().to_string()))
            } else {
                Error::internal_error().data(json!({
                    "path": resolved.display().to_string(),
                    "error": err.to_string(),
                }))
            }
        })?;

        let sliced = Self::slice_lines(&content, args.line, args.limit);
        Ok(ReadTextFileResponse::new(sliced))
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

                let tool_name = Self::tool_name_from_title(&call.title)
                    .map(|value| value.to_string())
                    .or_else(|| {
                        call.raw_input
                            .as_ref()
                            .and_then(Self::tool_name_from_payload)
                    });

                if let Some(name) = &tool_name {
                    self.record_tool_call_name(&call.tool_call_id, name);
                }

                let is_return_task_tool = matches!(tool_name.as_deref(), Some("return_task"))
                    || call.title.contains("return_task");
                let is_finalize_tool = matches!(tool_name.as_deref(), Some("finalize_review"))
                    || call.title.contains("finalize_review");

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
                        && Self::payload_has_title(input)
                    {
                        self.mark_finalization_received();
                    }
                    if let Some(ref output) = call.raw_output
                        && Self::payload_has_title(output)
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
                let tool_name = update
                    .fields
                    .title
                    .as_deref()
                    .and_then(Self::tool_name_from_title)
                    .map(|value| value.to_string())
                    .or_else(|| {
                        update
                            .fields
                            .raw_input
                            .as_ref()
                            .and_then(Self::tool_name_from_payload)
                    })
                    .or_else(|| self.lookup_tool_call_name(&update.tool_call_id));
                let is_completed = matches!(
                    update.fields.status,
                    Some(agent_client_protocol::ToolCallStatus::Completed)
                );
                let is_failed = matches!(
                    update.fields.status,
                    Some(agent_client_protocol::ToolCallStatus::Failed)
                );
                let is_finalize = matches!(tool_name.as_deref(), Some("finalize_review"))
                    || tool_id.contains("finalize_review");

                if is_finalize && is_completed {
                    if std::env::var("ACP_DEBUG").is_ok() {
                        eprintln!("[acp] finalize_review completed via ToolCallUpdate");
                    }
                    self.mark_finalization_received();
                }

                if is_completed || is_failed {
                    self.clear_tool_call_name(&update.tool_call_id);
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
