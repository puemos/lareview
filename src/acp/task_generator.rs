//! Task generator module for LaReview
//! Handles communication with ACP (Agent Client Protocol) agents to generate
//! review tasks from git diffs using AI agents like Codex, Qwen, and Gemini.

use crate::domain::{PullRequest, ReviewTask};
use crate::prompts;
use agent_client_protocol::{
    Agent, ClientCapabilities, ClientSideConnection, ContentBlock, ExtNotification, ExtRequest,
    ExtResponse, FileSystemCapability, Implementation, InitializeRequest, McpServer,
    McpServerStdio, Meta, NewSessionRequest, PermissionOptionKind, PromptRequest, ProtocolVersion,
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SelectedPermissionOutcome, SessionNotification, SessionUpdate, TextContent, ToolKind,
};
use anyhow::{Context as _, Result};
use async_trait::async_trait;
use futures::future::LocalBoxFuture;
use serde_json::json;
use serde_json::value::RawValue;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::runtime::Builder;
use tokio::task::LocalSet;

/// Input parameters for task generation
pub struct GenerateTasksInput {
    /// Pull request context for the task generation
    pub pull_request: PullRequest,
    /// Git diff text to analyze and generate tasks for
    pub diff_text: String,
    /// Optional repository root for read-only context
    ///
    /// When this is None, the agent must operate diff-only without filesystem or terminal access.
    /// When Some, the agent may read files under this root for context only.
    pub repo_root: Option<PathBuf>,
    /// Command to execute the ACP agent
    pub agent_command: String,
    /// Arguments to pass to the ACP agent command
    pub agent_args: Vec<String>,
    /// Optional channel to send progress updates during generation
    pub progress_tx: Option<tokio::sync::mpsc::UnboundedSender<ProgressEvent>>,
    /// Override for MCP server binary path
    pub mcp_server_binary: Option<PathBuf>,
    /// Timeout in seconds for agent execution
    pub timeout_secs: Option<u64>,
    /// Enable debug logging
    pub debug: bool,
}

/// Result of task generation
#[derive(Debug)]
pub struct GenerateTasksResult {
    pub messages: Vec<String>,
    pub thoughts: Vec<String>,
    pub logs: Vec<String>,
}

/// Different types of progress updates that can be streamed from the agent
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// Raw ACP session update, streamed to the UI.
    Update(Box<SessionUpdate>),
    /// Local log output from the ACP worker/process.
    LocalLog(String),
}

/// Client implementation for receiving agent callbacks
struct LaReviewClient {
    messages: Arc<Mutex<Vec<String>>>,
    thoughts: Arc<Mutex<Vec<String>>>,
    tasks: Arc<Mutex<Option<Vec<ReviewTask>>>>,
    raw_tasks_payload: Arc<Mutex<Option<serde_json::Value>>>,
    last_message_id: Arc<Mutex<Option<String>>>,
    last_thought_id: Arc<Mutex<Option<String>>>,
    progress: Option<tokio::sync::mpsc::UnboundedSender<ProgressEvent>>,
    pr_id: String,
    has_repo_access: bool,
    repo_root: Option<PathBuf>,
}

impl LaReviewClient {
    fn new(
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
        let parsed = crate::acp::task_mcp_server::parse_tasks(value.clone());
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
        let is_return_tool = tool_title.contains("return_tasks") || tool_title.contains("return_plans");
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

                let is_task_tool = call.title.contains("return_tasks")
                    || call.title.contains("create_review_tasks");

                if is_task_tool {
                    if let Some(ref input) = call.raw_input {
                        self.store_tasks_from_value(input.clone());
                    }
                    if let Some(ref output) = call.raw_output {
                        self.store_tasks_from_value(output.clone());
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

fn push_log(logs: &Arc<Mutex<Vec<String>>>, message: impl Into<String>, debug: bool) {
    let msg = message.into();
    if let Ok(mut guard) = logs.lock() {
        guard.push(msg.clone());
    }
    // Fire streaming progress if available (best-effort)
    if debug {
        eprintln!("[acp] {msg}");
    }
}

fn build_prompt(pr: &PullRequest, diff_text: &str, repo_root: Option<&PathBuf>) -> String {
    let has_repo_access = repo_root.is_some();
    prompts::render(
        "generate_tasks",
        &json!({
            "id": pr.id,
            "title": pr.title,
            "repo": pr.repo,
            "author": pr.author,
            "branch": pr.branch,
            "diff": diff_text,
            "has_repo_access": has_repo_access,
            "repo_root": repo_root.map(|p| p.display().to_string()),
            "repo_access_note": if has_repo_access { "read-only" } else { "none" }
        }),
    )
    .expect("failed to render generate_tasks prompt")
}

fn build_client_capabilities(has_repo_access: bool) -> ClientCapabilities {
    let fs_cap = if has_repo_access {
        FileSystemCapability::new()
            .read_text_file(true)
            .write_text_file(false)
    } else {
        FileSystemCapability::new()
            .read_text_file(false)
            .write_text_file(false)
    };

    ClientCapabilities::new()
        .fs(fs_cap)
        .terminal(false)
        .meta(Meta::from_iter([
            (
                "lareview-return-tasks".into(),
                serde_json::json!({
                    "type": "extension",
                    "method": "lareview/return_tasks",
                    "description": "Submit review tasks back to the client as structured data",
                    "params": {
                        "tasks": [{
                            "id": "string",
                            "title": "string",
                            "description": "string",
                            "files": ["string"],
                            "stats": {
                                "additions": "number",
                                "deletions": "number",
                                "risk": "LOW|MEDIUM|HIGH",
                                "tags": ["string"]
                            },
                            "patches": [{"file": "string", "hunk": "string"}]
                        }]
                    }
                }),
            ),
            (
                "lareview-return-plans".into(),
                serde_json::json!({
                    "type": "extension",
                    "method": "lareview/return_plans",
                    "description": "Submit review plans back to the client as structured data",
                    "params": {
                        "plans": [{
                            "entries": [{
                                "content": "string",
                                "priority": "LOW|MEDIUM|HIGH",
                                "status": "PENDING|IN_PROGRESS|COMPLETED",
                                "meta": "object"
                            }],
                            "meta": "object"
                        }]
                    }
                }),
            ),
        ]))
}

fn normalize_task_path(path: &str) -> String {
    path.trim()
        .trim_start_matches("./")
        .trim_start_matches("a/")
        .trim_start_matches("b/")
        .to_string()
}

fn extract_changed_files(diff_text: &str) -> HashSet<String> {
    let mut files = HashSet::new();
    for line in diff_text.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            let mut parts = rest.split_whitespace();
            let a_path = parts.next().unwrap_or("");
            let b_path = parts.next().unwrap_or("");
            if b_path.is_empty() {
                continue;
            }

            let b_clean = normalize_task_path(b_path);
            if b_clean == "dev/null" || b_clean == "/dev/null" {
                let a_clean = normalize_task_path(a_path);
                if !a_clean.is_empty() && a_clean != "dev/null" && a_clean != "/dev/null" {
                    files.insert(a_clean);
                }
            } else if !b_clean.is_empty() {
                files.insert(b_clean);
            }
        }
    }
    files
}

fn validate_tasks_payload(
    tasks: &[ReviewTask],
    raw_payload: Option<&serde_json::Value>,
    diff_text: &str,
) -> Result<Vec<String>> {
    if tasks.len() < 2 || tasks.len() > 7 {
        anyhow::bail!("return_tasks must provide 2-7 tasks, got {}", tasks.len());
    }

    if let Some(raw) = raw_payload {
        let tasks_arr = raw
            .get("tasks")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("return_tasks payload missing tasks array"))?;
        for (idx, t) in tasks_arr.iter().enumerate() {
            let risk_str = t
                .get("stats")
                .and_then(|s| s.get("risk"))
                .and_then(|r| r.as_str())
                .map(|s| s.to_uppercase());
            match risk_str.as_deref() {
                Some("LOW") | Some("MEDIUM") | Some("HIGH") | Some("MED") => {}
                Some(other) => anyhow::bail!("Task {idx} has invalid stats.risk '{other}'"),
                None => anyhow::bail!("Task {idx} missing stats.risk"),
            }
        }
    }

    let changed_files = extract_changed_files(diff_text);
    let mentioned_files: HashSet<String> = tasks
        .iter()
        .flat_map(|task| task.files.iter())
        .map(|f| normalize_task_path(f))
        .collect();

    let missing: Vec<String> = changed_files
        .difference(&mentioned_files)
        .cloned()
        .collect();
    if !missing.is_empty() {
        anyhow::bail!(
            "Tasks do not cover all changed files. Missing: {}",
            missing.join(", ")
        );
    }

    // Optional: ensure task diffs are substrings of the provided diff.
    let mut warnings = Vec::new();
    let diff_norm = diff_text.replace("\r\n", "\n");
    for task in tasks {
        if task.diffs.iter().any(|d| !diff_norm.contains(d)) {
            warnings.push(format!(
                "Task {} includes diffs not found in provided <diff>",
                task.id
            ));
        }
    }

    Ok(warnings)
}

/// Resolve the binary to launch for the MCP task server, preferring explicit overrides.
fn resolve_task_mcp_server_path(override_path: Option<&PathBuf>, current_exe: &Path) -> PathBuf {
    if let Some(path) = override_path {
        return path.clone();
    }
    if let Some(path) = option_env!("CARGO_BIN_EXE_lareview") {
        return PathBuf::from(path);
    }
    current_exe.to_path_buf()
}

/// Run ACP task generation on a dedicated Tokio runtime so GPUI stays responsive.
pub async fn generate_tasks_with_acp(input: GenerateTasksInput) -> Result<GenerateTasksResult> {
    let (sender, receiver) = futures::channel::oneshot::channel();
    let timeout_secs = input.timeout_secs.unwrap_or(5000);

    thread::spawn(move || {
        let runtime = Builder::new_current_thread().enable_all().build();
        let result = match runtime {
            Ok(rt) => {
                let local = LocalSet::new();
                local.block_on(&rt, async move {
                    tokio::time::timeout(
                        Duration::from_secs(timeout_secs),
                        generate_tasks_with_acp_inner(input),
                    )
                    .await
                    .map_err(|_| {
                        anyhow::anyhow!(format!("Agent timed out after {timeout_secs}s"))
                    })?
                })
            }
            Err(e) => Err(e.into()),
        };

        let _ = sender.send(result);
    });

    receiver
        .await
        .unwrap_or_else(|_| Err(anyhow::anyhow!("ACP worker thread unexpectedly closed")))
}

/// Generate review tasks using ACP agent (runs inside Tokio runtime).
async fn generate_tasks_with_acp_inner(input: GenerateTasksInput) -> Result<GenerateTasksResult> {
    let GenerateTasksInput {
        pull_request,
        diff_text,
        repo_root,
        agent_command,
        agent_args,
        progress_tx,
        mcp_server_binary,
        timeout_secs: _,
        debug,
    }: GenerateTasksInput = input;

    let logs = Arc::new(Mutex::new(Vec::new()));
    let progress_tx = progress_tx;

    let has_repo_access = repo_root.is_some();

    // Spawn agent process
    let progress_tx_for_log = progress_tx.clone();
    let log_fn = |msg: String| {
        push_log(&logs, &msg, debug);
        if let Some(tx) = &progress_tx_for_log {
            let _ = tx.send(ProgressEvent::LocalLog(msg));
        }
    };

    log_fn(format!("spawn: {} {}", agent_command, agent_args.join(" ")));

    let mut child = Command::new(&agent_command)
        .args(&agent_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| {
            format!(
                "Failed to spawn agent process: {} {}",
                agent_command,
                agent_args.join(" ")
            )
        })?;

    log_fn(format!("spawned pid: {}", child.id().unwrap_or(0)));

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to get stdin"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to get stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to get stderr"))?;

    let logs_clone = logs.clone();
    let progress_tx_for_stderr = progress_tx.clone();
    tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let msg = format!("stderr: {line}");
            push_log(&logs_clone, &msg, debug);
            if let Some(tx) = &progress_tx_for_stderr {
                let _ = tx.send(ProgressEvent::LocalLog(msg));
            }
        }
    });

    // Create client and connection
    let client = LaReviewClient::new(
        progress_tx.clone(),
        pull_request.id.clone(),
        repo_root.clone(),
    );
    let messages = client.messages.clone();
    let thoughts = client.thoughts.clone();
    let tasks_capture = client.tasks.clone();
    let raw_tasks_capture = client.raw_tasks_payload.clone();

    // Convert tokio streams for ACP
    use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
    let stdin_compat = stdin.compat_write();
    let stdout_compat = stdout.compat();

    let spawn_fn = |fut: LocalBoxFuture<'static, ()>| {
        tokio::task::spawn_local(fut);
    };

    let (connection, io_future) =
        ClientSideConnection::new(client, stdin_compat, stdout_compat, spawn_fn);

    // Spawn IO task
    let io_handle = tokio::task::spawn_local(async move {
        let _ = io_future.await;
    });

    // Initialize connection with chained builder pattern
    push_log(&logs, "initialize", debug);
    connection
        .initialize(
            InitializeRequest::new(ProtocolVersion::V1)
                .client_info(Implementation::new("lareview", "0.1.0"))
                .client_capabilities(build_client_capabilities(has_repo_access)),
        )
        .await
        .with_context(|| "ACP initialize failed")?;
    push_log(&logs, "initialize ok", debug);

    // Create session with MCP task server
    let temp_cwd = if has_repo_access {
        None
    } else {
        Some(tempfile::tempdir().context("create temp working directory")?)
    };
    let cwd = match &repo_root {
        Some(root) => root.clone(),
        None => temp_cwd
            .as_ref()
            .expect("temp_cwd present when no repo access")
            .path()
            .to_path_buf(),
    };

    let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("lareview"));
    let task_mcp_server_path =
        resolve_task_mcp_server_path(mcp_server_binary.as_ref(), &current_exe);

    let pr_context_file =
        tempfile::NamedTempFile::new().context("create PR context file for MCP server")?;
    std::fs::write(
        pr_context_file.path(),
        serde_json::to_string(&pull_request).context("serialize PR context")?,
    )
    .context("write PR context file")?;

    let mut mcp_args = vec![
        "--task-mcp-server".to_string(),
        "--pr-context".to_string(),
        pr_context_file.path().to_string_lossy().to_string(),
    ];
    if let Ok(db_path) = std::env::var("LAREVIEW_DB_PATH") {
        mcp_args.push("--db-path".to_string());
        mcp_args.push(db_path);
    }

    let mcp_servers = vec![McpServer::Stdio(
        McpServerStdio::new("lareview-tasks", task_mcp_server_path.clone()).args(mcp_args),
    )];

    log_fn(format!(
        "new_session (mcp server: {} --task-mcp-server --pr-context ...)",
        task_mcp_server_path.display(),
    ));
    let session = connection
        .new_session(NewSessionRequest::new(cwd).mcp_servers(mcp_servers))
        .await
        .with_context(|| "ACP new_session failed")?;
    push_log(&logs, "new_session ok", debug);

    // Send prompt
    let prompt_text = build_prompt(&pull_request, &diff_text, repo_root.as_ref());
    push_log(&logs, "prompt", debug);
    let _result = connection
        .prompt(PromptRequest::new(
            session.session_id,
            vec![ContentBlock::Text(TextContent::new(prompt_text))],
        ))
        .await
        .with_context(|| "ACP prompt failed")?;
    push_log(&logs, "prompt ok", debug);

    // Wait for the agent to finish naturally. If tasks are captured, we allow a short
    // grace period and then terminate to avoid hanging the UI on agents that don't exit.
    let status = loop {
        let tasks_ready = tasks_capture
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|t| !t.is_empty());

        if tasks_ready {
            let wait_res = tokio::time::timeout(Duration::from_secs(2), child.wait()).await;
            break match wait_res {
                Ok(res) => res?,
                Err(_) => {
                    push_log(
                        &logs,
                        "tasks captured but agent still running; sending kill",
                        debug,
                    );
                    let _ = child.start_kill();
                    child.wait().await?
                }
            };
        }

        tokio::select! {
            res = child.wait() => {
                break res?;
            }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {}
        }
    };
    // Wait for IO loop to flush pending notifications
    let _ = io_handle.await;

    // Snapshot artifacts before reading tasks

    push_log(&logs, format!("Agent exit status: {}", status), debug);

    // Prefer tasks captured via MCP callbacks, fall back to file written by MCP server.
    let final_tasks = tasks_capture.lock().unwrap().clone().unwrap_or_default();

    // Collect output for return and for richer error contexts
    let final_messages = messages.lock().unwrap().clone();
    let final_thoughts = thoughts.lock().unwrap().clone();

    if final_tasks.is_empty() {
        let final_logs = logs.lock().unwrap().clone();
        let ctx_logs = if final_logs.is_empty() {
            "ACP invocation produced no stderr or phase logs".to_string()
        } else {
            format!("ACP invocation logs:\n{}", final_logs.join("\n"))
        };
        let ctx_messages = if final_messages.is_empty() {
            String::new()
        } else {
            format!("Agent messages:\n{}", final_messages.join("\n"))
        };
        let ctx_thoughts = if final_thoughts.is_empty() {
            String::new()
        } else {
            format!("Agent thoughts:\n{}", final_thoughts.join("\n"))
        };

        let mut ctx_parts = vec![ctx_logs];
        if !ctx_messages.is_empty() {
            ctx_parts.push(ctx_messages);
        }
        if !ctx_thoughts.is_empty() {
            ctx_parts.push(ctx_thoughts);
        }

        return Err(anyhow::anyhow!(
            "Agent completed but did not call MCP tool return_tasks (no tasks captured)"
        )
        .context(ctx_parts.join("\n\n")));
    }

    let raw_payload = raw_tasks_capture.lock().unwrap().clone();
    let warnings = validate_tasks_payload(&final_tasks, raw_payload.as_ref(), &diff_text)?;
    for warning in warnings {
        push_log(&logs, format!("validation warning: {warning}"), debug);
    }

    let final_logs = logs.lock().unwrap().clone();
    Ok(GenerateTasksResult {
        messages: final_messages,
        thoughts: final_thoughts,
        logs: final_logs,
    })
}

#[cfg(test)]
mod policy_tests {
    use super::*;

    fn sample_pr() -> PullRequest {
        PullRequest {
            id: "pr-1".into(),
            title: "Test".into(),
            description: None,
            repo: "example/repo".into(),
            author: "tester".into(),
            branch: "main".into(),
            created_at: String::new(),
        }
    }

    fn sample_task(id: &str, files: &[&str]) -> ReviewTask {
        ReviewTask {
            id: id.into(),
            pr_id: "pr-1".into(),
            title: id.into(),
            description: String::new(),
            files: files.iter().map(|f| f.to_string()).collect(),
            stats: crate::domain::TaskStats {
                additions: 0,
                deletions: 0,
                risk: crate::domain::RiskLevel::Low,
                tags: vec![],
            },
            diffs: vec![],
            insight: None,
            diagram: None,
            ai_generated: true,
            status: crate::domain::TaskStatus::Pending,
            sub_flow: None,
        }
    }

    #[test]
    fn prompt_renders_no_repo_access_block() {
        let pr = sample_pr();
        let diff = "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n";
        let prompt = build_prompt(&pr, diff, None);
        assert!(prompt.contains("You do NOT have repository access."));
        assert!(prompt.contains("Do NOT call any tools except `return_tasks`."));
        assert!(!prompt.contains("You have READ-ONLY access"));
    }

    #[test]
    fn prompt_renders_repo_access_block() {
        let pr = sample_pr();
        let diff = "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n";
        let root = PathBuf::from("/tmp/repo-root");
        let prompt = build_prompt(&pr, diff, Some(&root));
        assert!(prompt.contains("You have READ-ONLY access"));
        assert!(prompt.contains(&root.display().to_string()));
        assert!(prompt.contains("Allowed tools:"));
        assert!(!prompt.contains("You do NOT have repository access."));
    }

    #[test]
    fn capabilities_disable_tools_without_repo() {
        let caps = build_client_capabilities(false);
        assert!(!caps.terminal);
        assert!(!caps.fs.read_text_file);
        assert!(!caps.fs.write_text_file);
    }

    #[test]
    fn capabilities_readonly_fs_with_repo() {
        let caps = build_client_capabilities(true);
        assert!(!caps.terminal);
        assert!(caps.fs.read_text_file);
        assert!(!caps.fs.write_text_file);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn permission_cancelled_without_repo_access() {
        let client = LaReviewClient::new(None, "pr-1", None);
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(ToolKind::Read)
            .title("fs/read_text_file")
            .raw_input(serde_json::json!({ "path": "src/a.rs" }));
        let tool_call = agent_client_protocol::ToolCallUpdate::new("tc1", fields);
        let options = vec![
            agent_client_protocol::PermissionOption::new(
                "allow",
                "Allow",
                PermissionOptionKind::AllowOnce,
            ),
            agent_client_protocol::PermissionOption::new(
                "reject",
                "Reject",
                PermissionOptionKind::RejectOnce,
            ),
        ];
        let req = RequestPermissionRequest::new(
            agent_client_protocol::SessionId::new("s1"),
            tool_call,
            options,
        );
        let resp =
            <LaReviewClient as agent_client_protocol::Client>::request_permission(&client, req)
                .await
                .unwrap();
        assert!(matches!(resp.outcome, RequestPermissionOutcome::Cancelled));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn permission_allows_return_tasks_even_without_repo_access() {
        let client = LaReviewClient::new(None, "pr-1", None);
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(ToolKind::Other)
            .title("return_tasks")
            .raw_input(serde_json::json!({ "tasks": [] }));
        let tool_call = agent_client_protocol::ToolCallUpdate::new("tc1", fields);
        let options = vec![agent_client_protocol::PermissionOption::new(
            "allow",
            "Allow",
            PermissionOptionKind::AllowOnce,
        )];
        let req =
            RequestPermissionRequest::new(agent_client_protocol::SessionId::new("s1"), tool_call, options);
        let resp =
            <LaReviewClient as agent_client_protocol::Client>::request_permission(&client, req)
                .await
                .unwrap();
        assert!(matches!(resp.outcome, RequestPermissionOutcome::Selected(_)));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn permission_allows_safe_read_under_repo_root() {
        let root = tempfile::tempdir().expect("root");
        let src_dir = root.path().join("src");
        std::fs::create_dir_all(&src_dir).expect("mkdir");
        std::fs::write(src_dir.join("a.rs"), "hi").expect("write");

        let client = LaReviewClient::new(None, "pr-1", Some(root.path().to_path_buf()));
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(ToolKind::Read)
            .title("fs/read_text_file")
            .raw_input(serde_json::json!({ "path": "src/a.rs" }));
        let tool_call = agent_client_protocol::ToolCallUpdate::new("tc1", fields);
        let options = vec![agent_client_protocol::PermissionOption::new(
            "allow",
            "Allow",
            PermissionOptionKind::AllowOnce,
        )];
        let req = RequestPermissionRequest::new(
            agent_client_protocol::SessionId::new("s1"),
            tool_call,
            options,
        );
        let resp =
            <LaReviewClient as agent_client_protocol::Client>::request_permission(&client, req)
                .await
                .unwrap();
        assert!(matches!(
            resp.outcome,
            RequestPermissionOutcome::Selected(_)
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn permission_denies_read_outside_repo_root() {
        let root = tempfile::tempdir().expect("root");
        std::fs::write(root.path().join("inside.rs"), "hi").expect("write");

        let client = LaReviewClient::new(None, "pr-1", Some(root.path().to_path_buf()));
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(ToolKind::Read)
            .title("fs/read_text_file")
            .raw_input(serde_json::json!({ "path": "../outside.rs" }));
        let tool_call = agent_client_protocol::ToolCallUpdate::new("tc1", fields);
        let options = vec![agent_client_protocol::PermissionOption::new(
            "allow",
            "Allow",
            PermissionOptionKind::AllowOnce,
        )];
        let req = RequestPermissionRequest::new(
            agent_client_protocol::SessionId::new("s1"),
            tool_call,
            options,
        );
        let resp =
            <LaReviewClient as agent_client_protocol::Client>::request_permission(&client, req)
                .await
                .unwrap();
        assert!(matches!(resp.outcome, RequestPermissionOutcome::Cancelled));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn permission_denies_execute_even_with_repo_access() {
        let root = tempfile::tempdir().expect("root");
        let client = LaReviewClient::new(None, "pr-1", Some(root.path().to_path_buf()));
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(ToolKind::Execute)
            .title("terminal/exec")
            .raw_input(serde_json::json!({ "command": "echo hi" }));
        let tool_call = agent_client_protocol::ToolCallUpdate::new("tc1", fields);
        let options = vec![agent_client_protocol::PermissionOption::new(
            "allow",
            "Allow",
            PermissionOptionKind::AllowOnce,
        )];
        let req = RequestPermissionRequest::new(
            agent_client_protocol::SessionId::new("s1"),
            tool_call,
            options,
        );
        let resp =
            <LaReviewClient as agent_client_protocol::Client>::request_permission(&client, req)
                .await
                .unwrap();
        assert!(matches!(resp.outcome, RequestPermissionOutcome::Cancelled));
    }

    #[test]
    fn validate_tasks_requires_full_file_coverage() {
        let diff = "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n";
        let tasks = vec![sample_task("T1", &["src/a.rs"]), sample_task("T2", &[])];
        let raw = serde_json::json!({
            "tasks": [
                { "stats": { "risk": "LOW" } },
                { "stats": { "risk": "LOW" } }
            ]
        });
        assert!(validate_tasks_payload(&tasks, Some(&raw), diff).is_ok());

        let missing = vec![sample_task("T1", &[]), sample_task("T2", &[])];
        assert!(validate_tasks_payload(&missing, Some(&raw), diff).is_err());
    }
}

#[cfg(test)]
mod mcp_config_tests {
    use super::*;

    #[test]
    fn resolve_prefers_mcp_server_binary_override() {
        let override_path = PathBuf::from("/tmp/custom-mcp-bin");
        let resolved = resolve_task_mcp_server_path(Some(&override_path), Path::new("/fallback"));
        assert_eq!(resolved, override_path);
    }
}

#[cfg(test)]
mod persistence_tests {}

#[cfg(test)]
mod real_acp_tests {
    use crate::data::{db::Database, repository::TaskRepository};

    use super::*;

    fn set_env(key: &str, val: &str) -> Option<String> {
        let prev = std::env::var(key).ok();
        unsafe {
            std::env::set_var(key, val);
        }
        prev
    }

    fn restore_env(key: &str, prev: Option<String>) {
        match prev {
            Some(val) => unsafe {
                std::env::set_var(key, val);
            },
            None => unsafe {
                std::env::remove_var(key);
            },
        }
    }

    /// Integration test: hits the real Codex ACP via npx.
    /// Run with: `cargo test -- --ignored`
    #[test]
    #[ignore]
    fn test_real_codex_acp_integration() {
        let diff = r#"diff --git a/src/beer.rs b/src/beer.rs
--- a/src/beer.rs
+++ b/src/beer.rs
@@ -1,23 +1,32 @@
 use std::time::Duration;

-#[derive(Debug)]
-pub struct BeerConfig {
-    pub brand: String,
-    pub temperature_c: u8,
-}
-
-pub fn open_bottle(brand: &str) {
-    println!("Opening {brand}");
-}
-
-pub fn chill(config: &BeerConfig) {
-    println!("Chilling {} to {}°C", config.brand, config.temperature_c);
-    std::thread::sleep(Duration::from_secs(3));
-}
-
-pub fn pour(brand: &str, ml: u32) {
-    println!("Pouring {ml}ml of {brand}");
-}
-
-pub fn drink(brand: &str, ml: u32) {
-    println!("Drinking {ml}ml of {brand}");
-}
+#[derive(Debug, Clone)]
+pub struct Beer {
+    brand: String,
+    temperature_c: u8,
+    opened: bool,
+}
+
+impl Beer {
+    pub fn new(brand: impl Into<String>, temperature_c: u8) -> Self {
+        Self {
+            brand: brand.into(),
+            temperature_c,
+            opened: false,
+        }
+    }
+
+    pub fn open(&mut self) {
+        if self.opened {
+            tracing::warn!("beer already open: {}", self.brand);
+            return;
+        }
+        self.opened = true;
+        println!("Opening {}", self.brand);
+    }
+
+    pub fn chill(&self) {
+        println!("Chilling {} to {}°C", self.brand, self.temperature_c);
+        std::thread::sleep(Duration::from_secs(3));
+    }
+
+    pub fn pour(&self, ml: u32) {
+        println!("Pouring {ml}ml of {}", self.brand);
+    }
+
+    pub fn drink(&self, ml: u32) {
+        println!("Drinking {ml}ml of {}", self.brand);
+    }
+}
"#;

        let pr = PullRequest {
            id: "test-pr".into(),
            title: "Test PR".into(),
            repo: "example/repo".into(),
            author: "tester".into(),
            branch: "main".into(),
            description: None,
            created_at: String::new(),
        };

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

        let input = GenerateTasksInput {
            pull_request: pr,
            diff_text: diff.to_string(),
            repo_root: None,
            agent_command: "npx".into(),
            agent_args: vec![
                "-y",
                "@zed-industries/codex-acp@latest",
                "-c",
                "model=\"gpt-5.1-codex-mini\"",
                "-c",
                "model_reasoning_effort=\"medium\"",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            progress_tx: Some(tx),
            mcp_server_binary: None,
            timeout_secs: Some(300),
            debug: true,
        };

        // Ensure we use the real binary, not the test harness
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir");
        let binary_path = std::path::PathBuf::from(manifest_dir).join("target/debug/lareview");
        if binary_path.exists() {
            unsafe {
                std::env::set_var("TASK_MCP_SERVER_BIN", binary_path);
            }
        } else {
            eprintln!(
                "WARNING: Real binary not found at {:?}, test might fail if using test harness",
                binary_path
            );
        }

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        let result = runtime.block_on(generate_tasks_with_acp(input));
        match &result {
            Ok(res) => {
                eprintln!("messages: {:#?}", res.messages);
                eprintln!("thoughts: {:#?}", res.thoughts);
                eprintln!("logs: {:#?}", res.logs);
            }
            Err(err) => eprintln!("error: {err:?}"),
        }
        assert!(
            result.is_ok(),
            "expected Codex ACP to return tasks: {:?}",
            result.err()
        );
    }

    /// Ignored by default: runs the real agent and asserts tasks were persisted to SQLite.
    /// Integration test with DB persistence.
    /// Run with: `cargo test -- --ignored`
    #[test]
    #[ignore]
    fn test_real_codex_acp_persist() -> anyhow::Result<()> {
        let tmp = tempfile::tempdir().expect("tmpdir");
        let db_path = tmp.path().join("db.sqlite");
        let prev_db = set_env("LAREVIEW_DB_PATH", db_path.to_string_lossy().as_ref());

        let diff = r#"diff --git a/src/foo.rs b/src/foo.rs
--- a/src/foo.rs
+++ b/src/foo.rs
@@ -1 +1,3 @@
-fn old() {}
+fn new_fn() {
+    println!("hi");
+}
"#;

        let pr = PullRequest {
            id: "test-pr".into(),
            title: "Test PR".into(),
            repo: "example/repo".into(),
            author: "tester".into(),
            branch: "main".into(),
            description: None,
            created_at: String::new(),
        };

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

        let input = GenerateTasksInput {
            pull_request: pr.clone(),
            diff_text: diff.to_string(),
            repo_root: None,
            agent_command: "npx".into(),
            agent_args: vec![
                "-y",
                "@zed-industries/codex-acp@latest",
                "-c",
                "model=\"gpt-5.1-codex-mini\"",
                "-c",
                "model_reasoning_effort=\"medium\"",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            progress_tx: Some(tx),
            mcp_server_binary: None,
            timeout_secs: Some(300),
            debug: true,
        };

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        let result = runtime.block_on(generate_tasks_with_acp(input))?;

        // Verify persisted tasks are present in SQLite
        let db = Database::open_at(db_path.clone())?;
        let repo = TaskRepository::new(db.connection());
        let tasks = repo.find_by_pr(&pr.id)?;
        assert!(
            !tasks.is_empty(),
            "expected tasks persisted, got none; logs: {:?}",
            result.logs
        );

        restore_env("LAREVIEW_DB_PATH", prev_db);
        Ok(())
    }
}
