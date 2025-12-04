//! Task generator - ACP client for generating review tasks

use crate::domain::{ParsedFileDiff, Patch, PullRequest, ReviewTask, RiskLevel, TaskStats};
use agent_client_protocol::{
    Agent, ClientSideConnection, ClientCapabilities, ContentBlock, Implementation, InitializeRequest,
    NewSessionRequest, PromptRequest, ProtocolVersion, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, SessionNotification, SessionUpdate, TextContent,
};
use anyhow::Result;
use async_trait::async_trait;
use futures::future::LocalBoxFuture;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Input for task generation
pub struct GenerateTasksInput {
    pub pull_request: PullRequest,
    pub files: Vec<ParsedFileDiff>,
    pub diff_text: Option<String>,
    pub agent_command: String,
    pub agent_args: Vec<String>,
}

/// Result of task generation
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
}

#[async_trait(?Send)]
impl agent_client_protocol::Client for LaReviewClient {
    async fn request_permission(
        &self,
        _args: RequestPermissionRequest,
    ) -> agent_client_protocol::Result<RequestPermissionResponse> {
        Ok(RequestPermissionResponse::new(RequestPermissionOutcome::Cancelled))
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
                if call.title.contains("return_tasks") || call.title.contains("task") {
                    if let Some(input) = call.raw_input {
                        if let Ok(parsed) = serde_json::from_value::<TasksArg>(input) {
                            let tasks = normalize_tasks(parsed.tasks);
                            *self.tasks.lock().unwrap() = Some(tasks);
                        }
                    }
                }
                // Also check raw_output for returned tasks
                if let Some(output) = call.raw_output {
                    if let Ok(parsed) = serde_json::from_value::<TasksArg>(output.clone()) {
                        let tasks = normalize_tasks(parsed.tasks);
                        *self.tasks.lock().unwrap() = Some(tasks);
                    }
                }
            }
            _ => {}
        }
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

/// Build prompt for task generation
fn build_prompt(pr: &PullRequest, diff_text: &str) -> String {
    format!(
        r#"You are a code review assistant. Analyze the following PR diff and return JSON with review tasks.

Pull request:
- id: {}
- title: {}
- repo: {}
- author: {}
- branch: {}

Unified diff:
{}

Return a JSON object with a "tasks" array. Each task should have:
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

/// Generate review tasks using ACP agent
pub async fn generate_tasks_with_acp(input: GenerateTasksInput) -> Result<GenerateTasksResult> {
    let diff_text = input.diff_text.unwrap_or_else(|| {
        input
            .files
            .iter()
            .map(|f| f.patch.clone())
            .collect::<Vec<_>>()
            .join("\n\n")
    });

    // Spawn agent process
    let mut child = Command::new(&input.agent_command)
        .args(&input.agent_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

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

    // Collect stderr logs
    let logs = Arc::new(Mutex::new(Vec::new()));
    let logs_clone = logs.clone();
    tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            logs_clone.lock().unwrap().push(line);
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
    connection
        .initialize(
            InitializeRequest::new(ProtocolVersion::LATEST)
                .client_info(Implementation::new("lareview", "0.1.0"))
                .client_capabilities(ClientCapabilities::new()),
        )
        .await?;

    // Create session
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let session = connection
        .new_session(NewSessionRequest::new(cwd))
        .await?;

    // Send prompt
    let prompt_text = build_prompt(&input.pull_request, &diff_text);
    let _result = connection
        .prompt(PromptRequest::new(
            session.session_id,
            vec![ContentBlock::Text(TextContent::new(prompt_text))],
        ))
        .await?;

    // Clean up
    drop(connection);
    let _ = io_handle.await;
    let _ = child.wait().await;

    // Get results - also try to parse tasks from messages
    let mut final_messages = std::mem::take(&mut *messages.lock().unwrap());
    let final_thoughts = std::mem::take(&mut *thoughts.lock().unwrap());
    let final_logs = std::mem::take(&mut *logs.lock().unwrap());
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

    Ok(GenerateTasksResult {
        tasks: final_tasks,
        messages: final_messages,
        thoughts: final_thoughts,
        logs: final_logs,
    })
}
