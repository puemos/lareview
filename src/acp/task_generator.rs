//! Task generator - ACP client for generating review tasks

use crate::domain::{ParsedFileDiff, Patch, PullRequest, ReviewTask, RiskLevel, TaskStats};
use agent_client_protocol::{
    Agent, ClientCapabilities, ClientSideConnection, ContentBlock, ExtNotification, ExtRequest,
    ExtResponse, FileSystemCapability, Implementation, InitializeRequest, Meta, NewSessionRequest,
    PromptRequest, ProtocolVersion, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, SelectedPermissionOutcome, SessionNotification, SessionUpdate,
    TextContent,
};
use anyhow::{Context as _, Result};
use async_trait::async_trait;
use futures::future::LocalBoxFuture;
use serde_json::value::RawValue;
use std::path::PathBuf;
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
    pub files: Vec<ParsedFileDiff>,
    pub diff_text: Option<String>,
    pub agent_command: String,
    pub agent_args: Vec<String>,
}

/// Result of task generation
#[derive(Debug)]
pub struct GenerateTasksResult {
    pub tasks: Vec<ReviewTask>,
    pub messages: Vec<String>,
    pub thoughts: Vec<String>,
    pub logs: Vec<String>,
}

/// Client implementation for receiving agent callbacks
struct LaReviewClient {
    messages: Arc<Mutex<Vec<String>>>,
    thoughts: Arc<Mutex<Vec<String>>>,
    tasks: Arc<Mutex<Option<Vec<ReviewTask>>>>,
}

impl LaReviewClient {
    fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
            thoughts: Arc::new(Mutex::new(Vec::new())),
            tasks: Arc::new(Mutex::new(None)),
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
        match notification.update {
            SessionUpdate::AgentMessageChunk(chunk) => {
                if let ContentBlock::Text(text) = chunk.content {
                    self.messages.lock().unwrap().push(text.text);
                }
            }
            SessionUpdate::AgentThoughtChunk(chunk) => {
                if let ContentBlock::Text(text) = chunk.content {
                    self.thoughts.lock().unwrap().push(text.text);
                }
            }
            SessionUpdate::ToolCall(call) => {
                // Check title for tool name and extract tasks from raw_input
                if (call.title.contains("return_tasks") || call.title.contains("task"))
                    && let Some(input) = call.raw_input
                {
                    self.store_tasks_from_value(input);
                }
                // Also check raw_output for returned tasks
                if let Some(output) = call.raw_output {
                    self.store_tasks_from_value(output);
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
            }
        })
        .collect()
}

fn push_log(logs: &Arc<Mutex<Vec<String>>>, message: impl Into<String>) {
    let msg = message.into();
    if let Ok(mut guard) = logs.lock() {
        guard.push(msg.clone());
    }
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

Preferred output: call the ACP extension method `lareview/return_tasks` (or `create_review_tasks`) with params:
{{"tasks": [{{ id, title, description, files, stats: {{ additions, deletions, risk: "LOW"|"MEDIUM"|"HIGH", tags: string[] }}, patches: [{{ file, hunk }}] }}]}}
If extensions/tools are unavailable, return the same data as a JSON object with a "tasks" array. Each task should have:
- id: unique string identifier
- title: short description of what to review
- description: detailed explanation
- files: array of affected file paths
- stats: {{ additions: number, deletions: number, risk: "LOW"|"MEDIUM"|"HIGH", tags: string[] }}
- patches: [{{ file: string, hunk: string }}]

Respond ONLY with the JSON object, no markdown or explanation."#,
        pr.id, pr.title, pr.repo, pr.author, pr.branch, diff_text
    )
}

/// Run ACP task generation on a dedicated Tokio runtime so GPUI stays responsive.
pub async fn generate_tasks_with_acp(input: GenerateTasksInput) -> Result<GenerateTasksResult> {
    let (sender, receiver) = futures::channel::oneshot::channel();
    let timeout_secs = std::env::var("ACP_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|s| *s > 0)
        .unwrap_or(90);

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
    let logs = Arc::new(Mutex::new(Vec::new()));
    let logs_for_error = logs.clone();

    let diff_text = input.diff_text.unwrap_or_else(|| {
        input
            .files
            .iter()
            .map(|f| f.patch.clone())
            .collect::<Vec<_>>()
            .join("\n\n")
    });

    // Spawn agent process
    push_log(
        &logs,
        format!(
            "spawn: {} {}",
            input.agent_command,
            input.agent_args.join(" ")
        ),
    );

    let mut child = Command::new(&input.agent_command)
        .args(&input.agent_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    push_log(&logs, format!("spawned pid: {}", child.id().unwrap_or(0)));

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
    tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            push_log(&logs_clone, format!("stderr: {line}"));
        }
    });

    // Create client and connection
    let client = LaReviewClient::new();
    let messages = client.messages.clone();
    let thoughts = client.thoughts.clone();
    let tasks = client.tasks.clone();

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
        .await?;
    push_log(&logs, "initialize ok");

    // Create session
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    push_log(&logs, "new_session");
    let session = connection
        .new_session(NewSessionRequest::new(cwd).mcp_servers(Vec::new()))
        .await?;
    push_log(&logs, "new_session ok");

    // Send prompt
    let prompt_text = build_prompt(&input.pull_request, &diff_text);
    push_log(&logs, "prompt");
    let _result = connection
        .prompt(PromptRequest::new(
            session.session_id,
            vec![ContentBlock::Text(TextContent::new(prompt_text))],
        ))
        .await?;
    push_log(&logs, "prompt ok");

    // Clean up
    drop(connection);
    io_handle.abort();

    let wait_res = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
    let status = match wait_res {
        Ok(res) => res?,
        Err(_) => {
            push_log(&logs, "agent did not exit after prompt; sending kill");
            let _ = child.start_kill();
            child.wait().await?
        }
    };

    // Get results - also try to parse tasks from messages
    let final_messages = std::mem::take(&mut *messages.lock().unwrap());
    let final_thoughts = std::mem::take(&mut *thoughts.lock().unwrap());
    let mut final_logs = std::mem::take(&mut *logs.lock().unwrap());
    let mut final_tasks = tasks.lock().unwrap().take().unwrap_or_default();

    // If no tasks from tool calls, try to parse from message text
    if final_tasks.is_empty() {
        for msg in &final_messages {
            if let Ok(parsed) = serde_json::from_str::<TasksArg>(msg) {
                final_tasks = normalize_tasks(parsed.tasks);
                break;
            }
        }
    }

    final_logs.push(format!("Agent exit status: {}", status));

    if !status.success() {
        return Err(anyhow::anyhow!(format!(
            "Agent exited with status {status}"
        )))
        .with_context(|| {
            let logs = logs_for_error.lock().unwrap();
            if logs.is_empty() {
                "ACP invocation failed without stderr output".to_string()
            } else {
                format!("ACP invocation failed. Agent stderr:\n{}", logs.join("\n"))
            }
        });
    }

    Ok(GenerateTasksResult {
        tasks: final_tasks,
        messages: final_messages,
        thoughts: final_thoughts,
        logs: final_logs,
    })
}

#[cfg(test)]
mod real_acp_tests {
    use super::*;

    /// Ignored by default: hits the real Codex ACP via npx when RUN_REAL_ACP=1.
    #[test]
    #[ignore]
    fn test_real_codex_acp_integration() {
        if std::env::var("RUN_REAL_ACP").as_deref() != Ok("1") {
            eprintln!("set RUN_REAL_ACP=1 to run this integration test");
            return;
        }

        let diff = r#"diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1 +1,2 @@
-fn main() {}
+fn main() {
+    println!("hello");
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

        let input = GenerateTasksInput {
            pull_request: pr,
            files: vec![],
            diff_text: Some(diff.to_string()),
            agent_command: "npx".into(),
            agent_args: vec!["-y", "@zed-industries/codex-acp@latest"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
        };

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
}
