//! Task generator - ACP client for generating review tasks

use crate::data::db::Database;
use crate::data::repository::{PullRequestRepository, TaskRepository};
use crate::domain::{Patch, PullRequest, ReviewTask, RiskLevel, TaskStats};
use agent_client_protocol::{
    Agent, ClientCapabilities, ClientSideConnection, ContentBlock, EnvVariable, ExtNotification,
    ExtRequest, ExtResponse, FileSystemCapability, Implementation, InitializeRequest, McpServer,
    McpServerStdio, Meta, NewSessionRequest, PromptRequest, ProtocolVersion,
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SelectedPermissionOutcome, SessionNotification, SessionUpdate, TextContent,
};
use anyhow::{Context as _, Result};
use async_trait::async_trait;
use futures::future::LocalBoxFuture;
use serde_json::value::RawValue;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::runtime::Builder;
use tokio::task::LocalSet;

/// Input for task generation
pub struct GenerateTasksInput {
    pub pull_request: PullRequest,
    pub diff_text: String,
    pub agent_command: String,
    pub agent_args: Vec<String>,
    pub progress_tx: Option<tokio::sync::mpsc::UnboundedSender<ProgressEvent>>,
}

/// Result of task generation
#[derive(Debug)]
pub struct GenerateTasksResult {
    pub tasks: Vec<ReviewTask>,
    pub messages: Vec<String>,
    pub thoughts: Vec<String>,
    pub logs: Vec<String>,
}

/// Streaming progress updates from the agent
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    Message { content: String, is_new: bool },
    Thought { content: String, is_new: bool },
    Log(String),
}

/// Client implementation for receiving agent callbacks
struct LaReviewClient {
    messages: Arc<Mutex<Vec<String>>>,
    thoughts: Arc<Mutex<Vec<String>>>,
    tasks: Arc<Mutex<Option<Vec<ReviewTask>>>>,
    last_message_id: Arc<Mutex<Option<String>>>,
    last_thought_id: Arc<Mutex<Option<String>>>,
    progress: Option<tokio::sync::mpsc::UnboundedSender<ProgressEvent>>,
}

impl LaReviewClient {
    fn new(progress: Option<tokio::sync::mpsc::UnboundedSender<ProgressEvent>>) -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
            thoughts: Arc::new(Mutex::new(Vec::new())),
            tasks: Arc::new(Mutex::new(None)),
            last_message_id: Arc::new(Mutex::new(None)),
            last_thought_id: Arc::new(Mutex::new(None)),
            progress,
        }
    }

    /// Attempt to parse and store tasks from arbitrary JSON value.
    fn store_tasks_from_value(&self, value: serde_json::Value) -> bool {
        if let Ok(parsed) = serde_json::from_value::<TasksArg>(value) {
            let tasks = normalize_tasks(parsed.tasks);
            *self.tasks.lock().unwrap() = Some(tasks);
            return true;
        }
        false
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
        let outcome = args
            .options
            .first()
            .map(|option| {
                RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(
                    option.option_id.clone(),
                ))
            })
            .unwrap_or(RequestPermissionOutcome::Cancelled);
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

        match notification.update {
            SessionUpdate::AgentMessageChunk(chunk) => {
                if let ContentBlock::Text(text) = chunk.content {
                    let (content, is_new) = self.append_streamed_content(
                        &self.messages,
                        &self.last_message_id,
                        chunk.meta.as_ref(),
                        &text.text,
                    );
                    if let Some(tx) = &self.progress {
                        let _ = tx.send(ProgressEvent::Message { content, is_new });
                    }
                }
            }
            SessionUpdate::AgentThoughtChunk(chunk) => {
                if let ContentBlock::Text(text) = chunk.content {
                    let (content, is_new) = self.append_streamed_content(
                        &self.thoughts,
                        &self.last_thought_id,
                        chunk.meta.as_ref(),
                        &text.text,
                    );
                    if let Some(tx) = &self.progress {
                        let _ = tx.send(ProgressEvent::Thought { content, is_new });
                    }
                }
            }
            SessionUpdate::ToolCall(ref call) => {
                // Debug log tool call details
                if std::env::var("ACP_DEBUG").is_ok() {
                    eprintln!(
                        "[acp] tool call: title={:?}, raw_input={:?}, raw_output={:?}",
                        call.title, call.raw_input, call.raw_output
                    );
                }

                // Check title for tool name and extract tasks from raw_input
                if call.title.contains("return_tasks") || call.title.contains("task") {
                    if let Some(ref input) = call.raw_input {
                        self.store_tasks_from_value(input.clone());
                        if let Some(tx) = &self.progress {
                            let _ = tx
                                .send(ProgressEvent::Log("received tool call return_tasks".into()));
                        }
                    }
                }
                // Also check raw_output for returned tasks
                if let Some(ref output) = call.raw_output {
                    self.store_tasks_from_value(output.clone());
                }
            }
            _ => {}
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

#[derive(serde::Deserialize)]
struct TasksArg {
    tasks: Vec<RawTask>,
}

#[derive(serde::Deserialize)]
struct RawTask {
    id: String,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    files: Vec<String>,
    #[serde(default)]
    stats: Option<RawStats>,
    #[serde(default)]
    patches: Vec<RawPatch>,
}

#[derive(serde::Deserialize, Default)]
struct RawStats {
    #[serde(default)]
    additions: u32,
    #[serde(default)]
    deletions: u32,
    #[serde(default)]
    risk: String,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(serde::Deserialize)]
struct RawPatch {
    file: String,
    hunk: String,
}

fn normalize_tasks(raw: Vec<RawTask>) -> Vec<ReviewTask> {
    raw.into_iter()
        .map(|t| {
            let stats = t.stats.unwrap_or_default();
            let risk = match stats.risk.to_uppercase().as_str() {
                "HIGH" => RiskLevel::High,
                "MEDIUM" | "MED" => RiskLevel::Medium,
                _ => RiskLevel::Low,
            };

            ReviewTask {
                id: t.id,
                title: t.title,
                description: t.description,
                files: t.files,
                stats: TaskStats {
                    additions: stats.additions,
                    deletions: stats.deletions,
                    risk,
                    tags: stats.tags,
                },
                patches: t
                    .patches
                    .into_iter()
                    .map(|p| Patch {
                        file: p.file,
                        hunk: p.hunk,
                    })
                    .collect(),
                insight: None,
                diagram: None,
                ai_generated: true,
                status: crate::domain::TaskStatus::default(),
            }
        })
        .collect()
}

fn push_log(logs: &Arc<Mutex<Vec<String>>>, message: impl Into<String>) {
    let msg = message.into();
    if let Ok(mut guard) = logs.lock() {
        guard.push(msg.clone());
    }
    // Fire streaming progress if available (best-effort)
    if std::env::var("ACP_DEBUG").is_ok() {
        eprintln!("[acp] {msg}");
    }
}

/// Build prompt for task generation
fn build_prompt(pr: &PullRequest, diff_text: &str) -> String {
    format!(
        r#"You are a code review assistant. Analyze the following PR diff and produce review tasks.

Pull request:
- id: {}
- title: {}
- repo: {}
- author: {}
- branch: {}

Unified diff:
{}

IMPORTANT MCP USAGE:
- There is an MCP server named "lareview-tasks" exposing a tool "return_tasks".
- Discover/confirm the server and tool (e.g., list MCP servers/tools) and use that tool.
- Do NOT run "return_tasks" as a shell command; it must be invoked as an MCP tool on "lareview-tasks".

The tool expects a JSON payload shaped as:
{{"tasks": [{{ "id": "string", "title": "string", "description": "string", "files": ["string"], "stats": {{ "additions": number, "deletions": number, "risk": "LOW"|"MEDIUM"|"HIGH", "tags": ["string"] }}, "patches": [{{ "file": "string", "hunk": "string" }}] }}]}}

After analyzing the diff, call the MCP tool return_tasks on server lareview-tasks with your structured review tasks. Do not output JSON to the terminal."#,
        pr.id, pr.title, pr.repo, pr.author, pr.branch, diff_text
    )
}

/// Resolve the binary to launch for the MCP task server, preferring explicit overrides.
fn resolve_task_mcp_server_path(current_exe: &Path) -> PathBuf {
    if let Ok(path) = std::env::var("TASK_MCP_SERVER_BIN") {
        return PathBuf::from(path);
    }
    if let Some(path) = option_env!("CARGO_BIN_EXE_lareview") {
        return PathBuf::from(path);
    }
    current_exe.to_path_buf()
}

/// Run ACP task generation on a dedicated Tokio runtime so GPUI stays responsive.
pub async fn generate_tasks_with_acp(input: GenerateTasksInput) -> Result<GenerateTasksResult> {
    let (sender, receiver) = futures::channel::oneshot::channel();
    let timeout_secs = std::env::var("ACP_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|s| *s > 0)
        .unwrap_or(500);

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
        agent_command,
        agent_args,
        progress_tx,
    } = input;

    let logs = Arc::new(Mutex::new(Vec::new()));
    let progress_tx = progress_tx;

    // Test hook: when LAREVIEW_FAKE_TASKS is set, skip ACP and persist provided tasks directly.
    if let Ok(fake_tasks) = std::env::var("LAREVIEW_FAKE_TASKS") {
        push_log(&logs, "fake_tasks: using LAREVIEW_FAKE_TASKS for testing");
        let parsed = serde_json::from_str::<TasksArg>(&fake_tasks)
            .map_err(|err| anyhow::anyhow!("Invalid LAREVIEW_FAKE_TASKS payload: {}", err))?;
        let final_tasks = normalize_tasks(parsed.tasks);
        persist_tasks_to_db(&pull_request, &final_tasks, &logs);
        return Ok(GenerateTasksResult {
            tasks: final_tasks,
            messages: Vec::new(),
            thoughts: Vec::new(),
            logs: logs.lock().unwrap().clone(),
        });
    }

    // Spawn agent process
    let progress_tx_for_log = progress_tx.clone();
    let log_fn = |msg: String| {
        push_log(&logs, &msg);
        if let Some(tx) = &progress_tx_for_log {
            let _ = tx.send(ProgressEvent::Log(msg));
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
            push_log(&logs_clone, &msg);
            if let Some(tx) = &progress_tx_for_stderr {
                let _ = tx.send(ProgressEvent::Log(msg));
            }
        }
    });

    // Create client and connection
    let client = LaReviewClient::new(progress_tx.clone());
    let messages = client.messages.clone();
    let thoughts = client.thoughts.clone();
    let tasks_capture = client.tasks.clone();

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
    push_log(&logs, "initialize");
    connection
        .initialize(
            InitializeRequest::new(ProtocolVersion::V1)
                .client_info(Implementation::new("lareview", "0.1.0"))
                .client_capabilities(
                    ClientCapabilities::new()
                        .fs(FileSystemCapability::new()
                            .read_text_file(true)
                            .write_text_file(true))
                        .terminal(true)
                        .meta(Meta::from_iter([
                            ("terminal_output".into(), serde_json::Value::Bool(true)),
                            ("terminal-auth".into(), serde_json::Value::Bool(true)),
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
                        ])),
                ),
        )
        .await
        .with_context(|| "ACP initialize failed")?;
    push_log(&logs, "initialize ok");

    // Create session with MCP task server
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let tasks_out_path = std::env::temp_dir().join(format!(
        "lareview_tasks_{}_{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));
    let mcp_log_path = std::env::temp_dir().join(format!(
        "lareview_tasks_log_{}_{}.log",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));
    let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("lareview"));
    let task_mcp_server_path = resolve_task_mcp_server_path(&current_exe);
    let mcp_servers = vec![McpServer::Stdio(
        McpServerStdio::new("lareview-tasks", task_mcp_server_path.clone())
            .args(vec!["--task-mcp-server".into()])
            .env(vec![
                EnvVariable::new("TASK_MCP_OUT", tasks_out_path.to_string_lossy().to_string()),
                EnvVariable::new("TASK_MCP_LOG", mcp_log_path.to_string_lossy().to_string()),
            ]),
    )];

    log_fn(format!(
        "new_session (mcp server: {} --task-mcp-server, out: {}, log: {})",
        task_mcp_server_path.display(),
        tasks_out_path.display(),
        mcp_log_path.display()
    ));
    let session = connection
        .new_session(NewSessionRequest::new(cwd).mcp_servers(mcp_servers))
        .await
        .with_context(|| "ACP new_session failed")?;
    push_log(&logs, "new_session ok");

    // Send prompt
    let prompt_text = build_prompt(&pull_request, &diff_text);
    push_log(&logs, "prompt");
    let _result = connection
        .prompt(PromptRequest::new(
            session.session_id,
            vec![ContentBlock::Text(TextContent::new(prompt_text))],
        ))
        .await
        .with_context(|| "ACP prompt failed")?;
    push_log(&logs, "prompt ok");

    let wait_res = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
    let status = match wait_res {
        Ok(res) => res?,
        Err(_) => {
            push_log(&logs, "agent did not exit after prompt; sending kill");
            let _ = child.start_kill();
            child.wait().await?
        }
    };
    // Wait for IO loop to flush pending notifications
    let _ = io_handle.await;

    // Snapshot artifacts before reading tasks
    let tasks_out_exists = tasks_out_path.exists();
    let tasks_out_size = std::fs::metadata(&tasks_out_path)
        .map(|m| m.len())
        .unwrap_or(0);
    let mcp_log_exists = mcp_log_path.exists();
    let mcp_log_size = std::fs::metadata(&mcp_log_path)
        .map(|m| m.len())
        .unwrap_or(0);

    push_log(&logs, format!("Agent exit status: {}", status));
    push_log(
        &logs,
        format!(
            "MCP artifacts: tasks_out_exists={} ({} bytes) {}, mcp_log_exists={} ({} bytes) {}",
            tasks_out_exists,
            tasks_out_size,
            tasks_out_path.display(),
            mcp_log_exists,
            mcp_log_size,
            mcp_log_path.display()
        ),
    );

    // Prefer tasks captured via MCP callbacks, fall back to file written by MCP server.
    let mut final_tasks = tasks_capture.lock().unwrap().clone().unwrap_or_default();
    if final_tasks.is_empty() {
        if let Ok(content) = std::fs::read_to_string(&tasks_out_path) {
            if let Ok(parsed) = serde_json::from_str::<TasksArg>(&content) {
                final_tasks = normalize_tasks(parsed.tasks);
            }
        }
    }

    // Collect output for return and for richer error contexts
    let final_messages = messages.lock().unwrap().clone();
    let final_thoughts = thoughts.lock().unwrap().clone();
    let final_logs = logs.lock().unwrap().clone();

    if final_tasks.is_empty() {
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

    persist_tasks_to_db(&pull_request, &final_tasks, &logs);

    Ok(GenerateTasksResult {
        tasks: final_tasks,
        messages: final_messages,
        thoughts: final_thoughts,
        logs: final_logs,
    })
}

fn persist_tasks_to_db(pr: &PullRequest, tasks: &[ReviewTask], logs: &Arc<Mutex<Vec<String>>>) {
    match Database::open() {
        Ok(db) => {
            let conn = db.connection();
            let pr_repo = PullRequestRepository::new(conn.clone());
            let task_repo = TaskRepository::new(conn.clone());

            if let Err(err) = pr_repo.save(pr) {
                push_log(logs, format!("db: failed to save PR: {err}"));
                return;
            }

            for task in tasks {
                if let Err(err) = task_repo.save(&pr.id, task) {
                    push_log(logs, format!("db: failed to save task {}: {err}", task.id));
                }
            }
            push_log(
                logs,
                format!(
                    "db: persisted {} tasks to sqlite at {}",
                    tasks.len(),
                    db.path().display()
                ),
            );
        }
        Err(err) => push_log(
            logs,
            format!("db: open failed, not persisting tasks: {err}"),
        ),
    }
}

#[cfg(test)]
mod mcp_config_tests {
    use super::*;
    use std::env;

    #[test]
    fn resolve_prefers_task_mcp_server_bin_env() {
        let original = env::var("TASK_MCP_SERVER_BIN").ok();
        unsafe {
            env::set_var("TASK_MCP_SERVER_BIN", "/tmp/custom-mcp-bin");
        }

        let resolved = resolve_task_mcp_server_path(Path::new("/fallback"));
        assert_eq!(resolved, PathBuf::from("/tmp/custom-mcp-bin"));

        if let Some(val) = original {
            unsafe {
                env::set_var("TASK_MCP_SERVER_BIN", val);
            }
        } else {
            unsafe {
                env::remove_var("TASK_MCP_SERVER_BIN");
            }
        }
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    use crate::data::db::Database;
    use crate::data::repository::TaskRepository;
    use crate::domain::TaskStatus;

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

    #[test]
    fn fake_tasks_env_persists_to_db() -> Result<()> {
        let tmp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = tmp_dir.path().join("db.sqlite");

        let prev_db = set_env("LAREVIEW_DB_PATH", db_path.to_string_lossy().as_ref());
        let prev_fake = set_env(
            "LAREVIEW_FAKE_TASKS",
            r#"{"tasks":[{"id":"task-1","title":"Test","description":"Desc","files":["a.rs"],"stats":{"additions":1,"deletions":0,"risk":"LOW","tags":[]}}]}"#,
        );

        let pr = PullRequest {
            id: "pr-1".into(),
            title: "Fake PR".into(),
            repo: "repo".into(),
            author: "me".into(),
            branch: "main".into(),
            description: None,
            created_at: "now".into(),
        };

        let input = GenerateTasksInput {
            pull_request: pr.clone(),
            diff_text: "diff --git a b".into(),
            agent_command: "true".into(),
            agent_args: vec![],
            progress_tx: None,
        };

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt");
        let res = rt.block_on(generate_tasks_with_acp(input))?;
        assert_eq!(res.tasks.len(), 1);

        // Verify persisted to DB
        let db = Database::open_at(db_path.clone())?;
        let repo = TaskRepository::new(db.connection());
        let stored = repo.find_by_pr(&pr.id)?;
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].title, "Test");
        assert_eq!(stored[0].status, TaskStatus::Pending);

        restore_env("LAREVIEW_DB_PATH", prev_db);
        restore_env("LAREVIEW_FAKE_TASKS", prev_fake);
        Ok(())
    }
}

#[cfg(test)]
mod real_acp_tests {
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

    /// Ignored by default: hits the real Codex ACP via npx when RUN_REAL_ACP=1.
    #[test]
    #[ignore]
    fn test_real_codex_acp_integration() {
        if std::env::var("RUN_REAL_ACP").as_deref() != Ok("1") {
            eprintln!("set RUN_REAL_ACP=1 to run this integration test");
            return;
        }

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
            .map(|s| s.to_string())
            .collect(),
            progress_tx: Some(tx),
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
                eprintln!("tasks: {:#?}", res.tasks);
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
    /// Set RUN_REAL_ACP_PERSIST=1 to run, and ensure the agent CLI is available.
    #[test]
    #[ignore]
    fn test_real_acp_persists_to_db() -> Result<()> {
        if std::env::var("RUN_REAL_ACP_PERSIST").as_deref() != Ok("1") {
            eprintln!("set RUN_REAL_ACP_PERSIST=1 to run this persistence integration");
            return Ok(());
        }

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
            .map(|s| s.to_string())
            .collect(),
            progress_tx: Some(tx),
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
