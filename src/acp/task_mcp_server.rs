#![allow(dead_code)]

//! Minimal MCP server that exposes a single `return_tasks` tool.
//!
//! The tool expects a JSON object `{ "tasks": [...] }` and writes it verbatim to
//! the path specified by the `TASK_MCP_OUT` environment variable. The server
//! runs over stdio so the ACP agent can launch it as an MCP server.

use crate::data::db::Database;
use crate::data::repository::{PullRequestRepository, TaskRepository};
use crate::domain::{PullRequest, ReviewTask, RiskLevel, TaskStats, TaskStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{Local, Utc};
use pmcp::error::TransportError;
use pmcp::shared::{Transport, TransportMessage};
use pmcp::{Server, ServerCapabilities, SimpleTool, ToolHandler};
use serde::Deserialize;
use serde_json::{Value, json};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Stdin, Stdout};
use tokio::sync::Mutex;

/// Configuration for the MCP server, parsed from CLI arguments
#[derive(Debug, Clone, Default)]
pub struct ServerConfig {
    /// Optional path to write tasks JSON output
    pub tasks_out: Option<PathBuf>,
    /// Optional path for debug log file
    pub log_file: Option<PathBuf>,
    /// Optional path to PR context JSON file
    pub pr_context: Option<PathBuf>,
    /// Optional explicit database path (for tests)
    pub db_path: Option<PathBuf>,
}

impl ServerConfig {
    /// Parse server configuration from command-line arguments
    pub fn from_args() -> Self {
        let mut config = ServerConfig::default();
        let args: Vec<String> = std::env::args().collect();
        let mut i = 0;

        while i < args.len() {
            match args[i].as_str() {
                "--log-file" => {
                    if i + 1 < args.len() {
                        config.log_file = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                _ => i += 1,
            }
        }

        config
    }
}

#[derive(Debug)]
struct LineDelimitedStdioTransport {
    stdin: Arc<Mutex<BufReader<Stdin>>>,
    stdout: Arc<Mutex<Stdout>>,
}

impl LineDelimitedStdioTransport {
    fn new() -> Self {
        Self {
            stdin: Arc::new(Mutex::new(BufReader::new(tokio::io::stdin()))),
            stdout: Arc::new(Mutex::new(tokio::io::stdout())),
        }
    }
}

#[async_trait]
impl Transport for LineDelimitedStdioTransport {
    async fn send(&mut self, message: TransportMessage) -> pmcp::Result<()> {
        let json = serde_json::to_string(&message)
            .map_err(|e| pmcp::Error::Transport(TransportError::Serialization(e.to_string())))?;

        let mut stdout = self.stdout.lock().await;
        stdout
            .write_all(json.as_bytes())
            .await
            .map_err(|e| pmcp::Error::Transport(TransportError::Io(e.to_string())))?;
        stdout
            .write_all(b"\n")
            .await
            .map_err(|e| pmcp::Error::Transport(TransportError::Io(e.to_string())))?;
        stdout
            .flush()
            .await
            .map_err(|e| pmcp::Error::Transport(TransportError::Io(e.to_string())))?;
        Ok(())
    }

    async fn receive(&mut self) -> pmcp::Result<TransportMessage> {
        let mut stdin = self.stdin.lock().await;
        let mut line = String::new();

        let bytes = stdin
            .read_line(&mut line)
            .await
            .map_err(|e| pmcp::Error::Transport(TransportError::Io(e.to_string())))?;

        if bytes == 0 {
            return Err(pmcp::Error::Transport(TransportError::ConnectionClosed));
        }

        let json_value: serde_json::Value = serde_json::from_str(&line).map_err(|e| {
            pmcp::Error::Transport(TransportError::InvalidMessage(format!(
                "Invalid JSON: {}",
                e
            )))
        })?;

        // Replicate pmcp::shared::StdioTransport::parse_message logic
        if json_value.get("method").is_some() {
            if json_value.get("id").is_some() {
                // Request
                let _request: pmcp::types::JSONRPCRequest<serde_json::Value> =
                    serde_json::from_value(json_value).map_err(|e| {
                        pmcp::Error::Transport(TransportError::InvalidMessage(format!(
                            "Invalid request: {}",
                            e
                        )))
                    })?;

                return pmcp::shared::StdioTransport::parse_message(line.as_bytes());
            } else {
                // Notification
                return pmcp::shared::StdioTransport::parse_message(line.as_bytes());
            }
        } else {
            // Response
            return pmcp::shared::StdioTransport::parse_message(line.as_bytes());
        }
    }

    async fn close(&mut self) -> pmcp::Result<()> {
        Ok(())
    }
}

/// Create the return_tasks tool with proper description and schema
fn create_return_tasks_tool(config: Arc<ServerConfig>) -> impl ToolHandler {
    SimpleTool::new("return_tasks", move |args: Value, _extra| {
        let config = config.clone();
        Box::pin(async move {
            log_to_file(&config, "return_tasks called");
            let persist_args = args.clone();
            let persist_config = config.clone();
            let persist_result = tokio::task::spawn_blocking(move || persist_tasks_to_db(&persist_config, persist_args)).await;

            match persist_result {
                Ok(Ok(())) => log_to_file(&config, "ReturnTasksTool persisted tasks to DB"),
                Ok(Err(err)) => log_to_file(&config, &format!("ReturnTasksTool failed to persist tasks: {err}")),
                Err(join_err) => log_to_file(&config, &format!("ReturnTasksTool task join error: {join_err}")),
            }

            if let Some(path) = &config.tasks_out {
                log_to_file(&config, &format!("ReturnTasksTool writing to {}", path.display()));
                // Best-effort write; if it fails we still return "ok" so the agent
                // doesn't treat it as a tool failure.
                let _ = std::fs::write(path, args.to_string());
                log_to_file(&config, "ReturnTasksTool write complete");
            }
            Ok(json!({ "status": "ok", "message": "Tasks received successfully" }))
        })
    })
    .with_description(
        "Submit code review tasks for a pull request. This tool finalizes your analysis. \
         Call it with a JSON payload containing a 'tasks' array where each task represents \
         a logical sub-flow or review concern from the PR diff. Each task must include: \
         id, title, description, files, stats (additions, deletions, risk, tags), and diffs. \
         Optionally include sub_flow (grouping name) and diagram (D2 format for visualization)."
    )
    .with_schema(json!({
        "type": "object",
        "properties": {
            "tasks": {
                "type": "array",
                "description": "Array of review tasks. Each task represents one logical sub-flow or review concern. CRITICAL: All tasks together must cover 100% of the diff - do not skip any changes.",
                "items": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Short stable identifier for the task. Prefer descriptive IDs that include the sub-flow (e.g., 'auth-T1-missing-tests', 'payment-flow-T1-logic-check') or generic IDs like 'T1', 'T2'"
                        },
                        "title": {
                            "type": "string",
                            "description": "One-line summary of the review task in imperative mood (e.g., 'Verify authentication flow changes', 'Review database migration logic')"
                        },
                        "description": {
                            "type": "string",
                            "description": "2-6 sentences explaining: (1) what this sub-flow does in the system, (2) what changed in this PR, (3) where it appears in the code, (4) why it matters (correctness/safety/performance), (5) what reviewers should verify"
                        },
                        "files": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "List of file paths that participate in this sub-flow. Include all relevant files, even if they span multiple directories. Paths should be relative to repo root (e.g., 'src/auth/login.rs', 'tests/auth_test.rs')"
                        },
                        "stats": {
                            "type": "object",
                            "description": "Statistics about the changes in this task. These can be approximate.",
                            "properties": {
                                "additions": {
                                    "type": "integer",
                                    "description": "Approximate number of lines added relevant to this task"
                                },
                                "deletions": {
                                    "type": "integer",
                                    "description": "Approximate number of lines deleted relevant to this task"
                                },
                                "risk": {
                                    "type": "string",
                                    "enum": ["LOW", "MEDIUM", "HIGH"],
                                    "description": "Risk level: HIGH for dangerous changes (security, data loss, breaking changes), MEDIUM for complex logic or refactors, LOW for safe mechanical changes"
                                },
                                "tags": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Descriptive tags for categorization (e.g., 'security', 'performance', 'refactor', 'bug-fix', 'needs-tests', 'breaking-change')"
                                }
                            },
                            "required": ["additions", "deletions", "risk", "tags"]
                        },
                        "sub_flow": {
                            "type": "string",
                            "description": "Optional logical grouping name for this task. Use when multiple tasks belong to the same larger feature or concern (e.g., 'authentication-flow', 'data-migration', 'payment-processing'). Helps organize related tasks."
                        },
                        "diagram": {
                            "type": "string",
                            "description": "STRONGLY RECOMMENDED: D2 diagram visualizing the flow, sequence, architecture, or data model. Create diagrams for MEDIUM/HIGH risk tasks or when multiple components interact. Must be valid D2 syntax (e.g., 'Client -> API: Request\\nAPI -> DB: Query')"
                        },
                        "diffs": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Array of complete unified diff strings showing the relevant changes for this task. CRITICAL: Each diff must be valid and parseable with full headers (diff --git a/file b/file, --- a/file, +++ b/file) and exact hunk headers (@@ -a,b +c,d @@) where line counts match precisely. Never approximate hunk ranges."
                        }
                    },
                    "required": ["id", "title", "description", "files", "stats", "diffs"]
                }
            }
        },
        "required": ["tasks"]
    }))
}

/// Run the MCP server over stdio. Blocks until the process is terminated.
pub async fn run_task_mcp_server() -> pmcp::Result<()> {
    let config = Arc::new(ServerConfig::from_args());
    log_to_file(&config, "starting task MCP server");

    let server = Server::builder()
        .name("lareview-tasks")
        .version("0.1.0")
        .capabilities(ServerCapabilities::default())
        .tool("return_tasks", create_return_tasks_tool(config.clone()))
        .build()?;

    log_to_file(&config, "running task MCP server on stdio (line-delimited)");
    let transport = LineDelimitedStdioTransport::new();
    server.run(transport).await
}

#[derive(Deserialize)]
struct TasksPayload {
    tasks: Vec<RawTask>,
}

#[derive(Deserialize)]
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
    diffs: Vec<String>,
    #[serde(default)]
    diagram: Option<String>,
    #[serde(default)]
    sub_flow: Option<String>,
}

#[derive(Deserialize, Default)]
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

fn persist_tasks_to_db(config: &ServerConfig, args: Value) -> Result<()> {
    let tasks = parse_tasks(args)?;
    let pull_request = load_pull_request(config);

    let db = match &config.db_path {
        Some(path) => Database::open_at(path.clone()),
        None => Database::open(),
    }
    .context("open database")?;
    let conn = db.connection();
    let pr_repo = PullRequestRepository::new(conn.clone());
    let task_repo = TaskRepository::new(conn);

    pr_repo.save(&pull_request).context("save pull request")?;

    for mut task in tasks {
        // Set the pr_id before saving the task
        task.pr_id = pull_request.id.clone();
        task_repo
            .save(&task)
            .with_context(|| format!("save task {}", task.id))?;
    }

    Ok(())
}

fn parse_tasks(args: Value) -> Result<Vec<ReviewTask>> {
    let payload: TasksPayload = serde_json::from_value(args)?;
    let tasks = payload
        .tasks
        .into_iter()
        .map(|task| {
            let stats = task.stats.unwrap_or_default();
            let risk = match stats.risk.to_uppercase().as_str() {
                "HIGH" => RiskLevel::High,
                "MEDIUM" | "MED" => RiskLevel::Medium,
                _ => RiskLevel::Low,
            };

            ReviewTask {
                id: task.id,
                pr_id: String::new(), // Will be set in persist_tasks_to_db
                title: task.title,
                description: task.description,
                files: task.files,
                stats: TaskStats {
                    additions: stats.additions,
                    deletions: stats.deletions,
                    risk,
                    tags: stats.tags,
                },
                diffs: task.diffs,
                insight: None,
                diagram: task.diagram,
                ai_generated: true,
                status: TaskStatus::Pending,
                sub_flow: task.sub_flow, // Use sub_flow from raw task
            }
        })
        .collect();

    Ok(tasks)
}

/// Load pull request context from file or return defaults
#[allow(clippy::collapsible_if)]
fn load_pull_request(config: &ServerConfig) -> PullRequest {
    // Try to load from --pr-context file if provided
    if let Some(path) = &config.pr_context {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(pr) = serde_json::from_str::<PullRequest>(&content) {
                return pr;
            }
        }
    }

    // Return defaults if no context file or parsing failed
    PullRequest {
        id: "local-pr".to_string(),
        title: "Review".to_string(),
        description: None,
        repo: "unknown/repo".to_string(),
        author: "unknown".to_string(),
        branch: "main".to_string(),
        created_at: Utc::now().to_rfc3339(),
    }
}

/// Generate the default log file path with date
fn default_log_path() -> PathBuf {
    let date = Local::now().format("%Y-%m-%d").to_string();
    let dir = PathBuf::from(".lareview/logs");
    let _ = fs::create_dir_all(&dir);
    dir.join(format!("mcp-{date}.log"))
}

/// Log a message to the configured log file, if any
fn log_to_file(_config: &ServerConfig, message: &str) {
    let path = default_log_path();
    let _ = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| writeln!(f, "{message}"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_util::sync::CancellationToken;

    #[tokio::test]
    async fn test_return_tasks_tool_writes_file() {
        let tmp = tempfile::NamedTempFile::new().expect("tmp file");
        let out_path = tmp.path().to_path_buf();
        let tmp_db = tempfile::tempdir().expect("tmp db dir");
        let db_path = tmp_db.path().join("db.sqlite");

        let config = Arc::new(ServerConfig {
            tasks_out: Some(out_path.clone()),
            log_file: None,
            pr_context: None,
            db_path: Some(db_path), // Use explicit db_path
        });

        let tool = create_return_tasks_tool(config);
        let payload = serde_json::json!({ "tasks": [{ "id": "x", "title": "test" }] });
        let res = tool
            .handle(
                payload.clone(),
                pmcp::RequestHandlerExtra::new("test".into(), CancellationToken::new()),
            )
            .await
            .expect("tool call ok");
        assert_eq!(
            res,
            serde_json::json!({ "status": "ok", "message": "Tasks received successfully" })
        );
        let written = std::fs::read_to_string(tmp.path()).expect("read tmp");
        assert_eq!(written, payload.to_string());
    }

    #[tokio::test]
    async fn test_return_tasks_tool_persists_to_db() {
        let tmp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = tmp_dir.path().join("db.sqlite");
        let pr_context_path = tmp_dir.path().join("pr.json");

        // Write PR context to file
        let pr_context = serde_json::json!({
            "id": "pr-db",
            "title": "Test PR",
            "description": null,
            "repo": "test/repo",
            "author": "tester",
            "branch": "main",
            "created_at": "2024-01-01T00:00:00Z"
        });
        std::fs::write(&pr_context_path, pr_context.to_string()).expect("write PR context");

        let config = Arc::new(ServerConfig {
            tasks_out: None,
            log_file: None,
            pr_context: Some(pr_context_path),
            db_path: Some(db_path.clone()), // Use explicit db_path
        });

        let tool = create_return_tasks_tool(config);
        let payload = serde_json::json!({
            "tasks": [{
                "id": "task-123",
                "title": "DB Task",
                "description": "persist me",
                "stats": { "risk": "HIGH" },
                "diffs": ["@@ -1 +1 @@"]
            }]
        });

        let _ = tool
            .handle(
                payload,
                pmcp::RequestHandlerExtra::new("test".into(), CancellationToken::new()),
            )
            .await
            .expect("tool call ok");

        // Give the spawn_blocking task time to complete persistence
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let db = Database::open_at(db_path.clone()).expect("open db");
        let repo = TaskRepository::new(db.connection());
        let tasks = repo.find_by_pr(&"pr-db".to_string()).expect("tasks for pr");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "task-123");
        assert_eq!(tasks[0].title, "DB Task");
        assert_eq!(tasks[0].status, TaskStatus::Pending);
    }
}
