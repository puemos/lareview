//! Learning compactor worker implementation.
//!
//! Spawns an ACP agent to analyze rejected feedback patterns and generate
//! learned patterns that guide future reviews.

use super::client::{LearningClient, LearningProgressEvent};
use crate::domain::{LearnedPattern, LearningCompactionResult};
use crate::infra::db::repository::FeedbackRejection;
use crate::prompts;
use agent_client_protocol::{
    Agent, ClientSideConnection, ContentBlock, Implementation, InitializeRequest, McpServer,
    McpServerStdio, NewSessionRequest, PromptRequest, ProtocolVersion, TextContent,
};
use anyhow::{Context, Result};
use futures::future::LocalBoxFuture;
use log::debug;
use serde_json::json;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Command;
use tokio::runtime::Builder;
use tokio::task::LocalSet;

/// Input for learning compaction
pub struct LearningCompactionInput {
    /// Unprocessed rejections to analyze
    pub rejections: Vec<FeedbackRejection>,
    /// Existing patterns for context (can be merged)
    pub existing_patterns: Vec<LearnedPattern>,
    /// Agent command to run (e.g., "claude")
    pub agent_command: String,
    /// Agent arguments
    pub agent_args: Vec<String>,
    /// Database to save results
    pub db: Arc<std::sync::Mutex<crate::infra::db::Database>>,
    /// Timeout in seconds
    pub timeout_secs: Option<u64>,
    /// Override for MCP server binary path.
    pub mcp_server_binary: Option<PathBuf>,
    /// Optional cancellation token.
    pub cancel_token: Option<tokio_util::sync::CancellationToken>,
    /// Enable debug logging.
    pub debug: bool,
}

#[cfg(unix)]
fn kill_process_group(pid: u32, logs: &Arc<Mutex<Vec<String>>>, debug: bool) {
    if pid == 0 {
        return;
    }
    let res = unsafe { libc::killpg(pid as i32, libc::SIGKILL) };
    if res != 0 {
        let err = std::io::Error::last_os_error();
        let raw = err.raw_os_error();
        let should_log = !matches!(raw, Some(code) if code == libc::EPERM || code == libc::ESRCH);
        if should_log {
            push_log(
                logs,
                format!("Failed to kill process group {}: {}", pid, err),
                debug,
            );
        }
    }
}

#[cfg(not(unix))]
fn kill_process_group(_pid: u32, _logs: &Arc<Mutex<Vec<String>>>, _debug: bool) {}

struct ProcessGroupGuard {
    pid: u32,
    logs: Arc<Mutex<Vec<String>>>,
    debug: bool,
    armed: bool,
}

impl ProcessGroupGuard {
    fn new(pid: u32, logs: Arc<Mutex<Vec<String>>>, debug: bool) -> Self {
        Self {
            pid,
            logs,
            debug,
            armed: true,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for ProcessGroupGuard {
    fn drop(&mut self) {
        if self.armed {
            kill_process_group(self.pid, &self.logs, self.debug);
        }
    }
}

fn push_log(logs: &Arc<Mutex<Vec<String>>>, message: impl Into<String>, debug_mode: bool) {
    let msg = message.into();
    if let Ok(mut guard) = logs.lock() {
        guard.push(msg.clone());
    }
    if debug_mode {
        debug!(target: "acp", "learning: {}", msg);
    }
}

/// Resolve the binary to launch for the MCP task server.
fn resolve_task_mcp_server_path(
    override_path: Option<&PathBuf>,
    current_exe: &std::path::Path,
) -> PathBuf {
    if let Some(path) = override_path {
        return path.clone();
    }
    if let Some(path) = option_env!("CARGO_BIN_EXE_lareview") {
        return PathBuf::from(path);
    }
    current_exe.to_path_buf()
}

/// Run learning compaction to analyze rejection patterns.
///
/// Uses the proper ACP pattern to spawn an agent with MCP server.
pub async fn run_learning_compaction(
    input: LearningCompactionInput,
) -> Result<LearningCompactionResult> {
    if input.rejections.is_empty() {
        return Ok(LearningCompactionResult {
            rejections_processed: 0,
            patterns_created: 0,
            patterns_updated: 0,
            errors: vec![],
        });
    }

    let (sender, receiver) = futures::channel::oneshot::channel();
    let timeout_secs = input.timeout_secs.unwrap_or(300);

    thread::spawn(move || {
        let runtime = Builder::new_current_thread().enable_all().build();
        let result = match runtime {
            Ok(rt) => {
                let local = LocalSet::new();
                local.block_on(&rt, async move {
                    let cancel_token = input.cancel_token.clone();
                    tokio::select! {
                        res = tokio::time::timeout(
                            Duration::from_secs(timeout_secs),
                            run_learning_compaction_inner(input),
                        ) => {
                            res.map_err(|_| {
                                anyhow::anyhow!("Learning agent timed out after {}s", timeout_secs)
                            })?
                        }
                        _ = async {
                            if let Some(token) = cancel_token {
                                token.cancelled().await;
                            } else {
                                std::future::pending::<()>().await;
                            }
                        } => {
                            Err(anyhow::anyhow!("Learning compaction cancelled by user"))
                        }
                    }
                })
            }
            Err(e) => Err(e.into()),
        };

        let _ = sender.send(result);
    });

    receiver.await.unwrap_or_else(|_| {
        Err(anyhow::anyhow!(
            "Learning worker thread unexpectedly closed"
        ))
    })
}

/// Inner implementation that runs on the dedicated runtime.
async fn run_learning_compaction_inner(
    input: LearningCompactionInput,
) -> Result<LearningCompactionResult> {
    let LearningCompactionInput {
        rejections,
        existing_patterns,
        agent_command,
        agent_args,
        db,
        timeout_secs: _,
        mcp_server_binary,
        cancel_token,
        debug,
    } = input;

    let logs = Arc::new(Mutex::new(Vec::new()));
    let rejections_count = rejections.len();
    let rejection_ids: Vec<String> = rejections.iter().map(|r| r.id.clone()).collect();

    // Build the prompt
    let prompt = build_learning_prompt(&rejections, &existing_patterns)?;

    let log_fn = |msg: String| {
        push_log(&logs, &msg, debug);
    };

    log_fn(format!("spawn: {} {}", agent_command, agent_args.join(" ")));

    // Spawn agent process
    let mut cmd = Command::new(&agent_command);
    cmd.args(&agent_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    #[cfg(unix)]
    {
        #[allow(unused_imports)]
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    let mut child = cmd.spawn().with_context(|| {
        format!(
            "Failed to spawn learning agent: {} {}",
            agent_command,
            agent_args.join(" ")
        )
    })?;

    let child_pid = child.id().unwrap_or(0);
    log_fn(format!("spawned pid: {}", child_pid));
    let mut process_guard = ProcessGroupGuard::new(child_pid, logs.clone(), debug);

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

    // Spawn a background task to monitor stderr
    tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let msg = format!("stderr: {line}");
            push_log(&logs_clone, &msg, debug);
        }
    });

    // Create client
    let (progress_tx, mut _progress_rx) =
        tokio::sync::mpsc::unbounded_channel::<LearningProgressEvent>();
    let client = LearningClient::new(Some(progress_tx));
    let patterns_capture = client.patterns.clone();
    let finalization_received_capture = client.finalization_received.clone();

    // Convert tokio streams for ACP
    use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
    let stdin_compat = stdin.compat_write();
    let stdout_compat = stdout.compat();

    let spawn_fn = |fut: LocalBoxFuture<'static, ()>| {
        tokio::task::spawn_local(fut);
    };

    let (connection, io_future) =
        ClientSideConnection::new(client, stdin_compat, stdout_compat, spawn_fn);

    // Spawn the asynchronous I/O loop
    let io_handle = tokio::task::spawn_local(async move {
        let _ = io_future.await;
    });

    let result = async {
        // Initialize connection
        push_log(&logs, "initialize", debug);
        connection
            .initialize(InitializeRequest::new(ProtocolVersion::V1).client_info(
                Implementation::new("lareview-learning", env!("CARGO_PKG_VERSION")),
            ))
            .await
            .with_context(|| "ACP initialize failed")?;
        push_log(&logs, "initialize ok", debug);

        // Create session with MCP task server
        let temp_cwd = tempfile::tempdir().context("create temp working directory")?;
        let cwd: PathBuf = temp_cwd.path().to_path_buf();

        let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("lareview"));
        let task_mcp_server_path =
            resolve_task_mcp_server_path(mcp_server_binary.as_ref(), &current_exe);

        // Build MCP server args - learning mode doesn't need PR context or repo root
        let mut mcp_args = vec!["--task-mcp-server".to_string()];
        if let Ok(db_path) = std::env::var("LAREVIEW_DB_PATH") {
            mcp_args.push("--db-path".to_string());
            mcp_args.push(db_path);
        }

        let mcp_servers = vec![McpServer::Stdio(
            McpServerStdio::new("lareview-tasks", task_mcp_server_path.clone()).args(mcp_args),
        )];

        push_log(
            &logs,
            format!(
                "new_session (mcp server: {} --task-mcp-server)",
                task_mcp_server_path.display(),
            ),
            debug,
        );
        let session = connection
            .new_session(NewSessionRequest::new(cwd).mcp_servers(mcp_servers))
            .await
            .with_context(|| "ACP new_session failed")?;
        push_log(&logs, "new_session ok", debug);

        // Send prompt
        push_log(&logs, "prompt", debug);
        let prompt_result = connection
            .prompt(PromptRequest::new(
                session.session_id,
                vec![ContentBlock::Text(TextContent::new(prompt))],
            ))
            .await;

        if let Err(err) = &prompt_result {
            push_log(&logs, format!("prompt error: {err:?}"), true);
            if let Ok(Some(status)) = child.try_wait() {
                push_log(
                    &logs,
                    format!("agent exited before prompt completed: {status}"),
                    true,
                );
            }

            if !*finalization_received_capture.lock().unwrap() {
                return Err(anyhow::anyhow!(
                    "ACP prompt failed: {:?}",
                    prompt_result.unwrap_err()
                ));
            }

            push_log(
                &logs,
                "prompt error ignored because finalization was received",
                debug,
            );
        } else {
            push_log(&logs, "prompt ok", debug);
        }

        // Monitor agent execution until completion or cancellation
        let status = loop {
            let finalization_received = *finalization_received_capture.lock().unwrap();

            if finalization_received {
                push_log(
                    &logs,
                    "finalization received; terminating agent immediately",
                    debug,
                );

                let _ = child.start_kill();
                kill_process_group(child_pid, &logs, debug);
                match child.wait().await {
                    Ok(res) => break res,
                    Err(e) => {
                        push_log(
                            &logs,
                            format!("failed to wait on child after kill: {}", e),
                            debug,
                        );
                        break child.wait().await?;
                    }
                }
            }

            if let Some(token) = &cancel_token
                && token.is_cancelled()
            {
                push_log(&logs, "cancellation received; killing agent", debug);
                let _ = child.start_kill();
                kill_process_group(child_pid, &logs, debug);
                let _ = child.wait().await;
                process_guard.disarm();
                return Err(anyhow::anyhow!("Learning compaction cancelled by user"));
            }

            tokio::select! {
                res = child.wait() => {
                    break res?;
                }
                _ = tokio::time::sleep(Duration::from_millis(200)) => {
                    if child.try_wait().unwrap_or(None).is_some() {
                        break child.wait().await?;
                    }
                }
            }
        };

        // Ensure all pending notifications are processed
        kill_process_group(child_pid, &logs, debug);
        let _ = io_handle.await;
        process_guard.disarm();

        push_log(&logs, format!("Agent exit status: {}", status), debug);

        let finalization_received = *finalization_received_capture.lock().unwrap();
        if !finalization_received {
            let final_logs = logs.lock().unwrap().clone();
            let ctx_logs = if final_logs.is_empty() {
                "ACP invocation produced no stderr or phase logs".to_string()
            } else {
                format!("ACP invocation logs:\n{}", final_logs.join("\n"))
            };

            return Err(
                anyhow::anyhow!("Agent completed but did not call finalize_learning")
                    .context(ctx_logs),
            );
        }

        Ok(())
    }
    .await;

    if let Err(e) = result {
        let _ = child.start_kill();
        kill_process_group(child_pid, &logs, debug);
        let _ = child.wait().await;
        process_guard.disarm();
        return Err(e);
    }

    // Collect patterns from client
    let collected_patterns = patterns_capture.lock().unwrap().clone();

    // Save patterns to database
    let mut final_result = LearningCompactionResult {
        rejections_processed: 0,
        patterns_created: 0,
        patterns_updated: 0,
        errors: vec![],
    };

    if collected_patterns.is_empty() {
        final_result
            .errors
            .push("Agent did not submit any patterns via submit_learned_patterns tool".to_string());
        return Ok(final_result);
    }

    // Save each pattern to the database
    {
        let db_guard = db.lock().map_err(|e| anyhow::anyhow!("db lock: {}", e))?;
        let pattern_repo = db_guard.learned_pattern_repo();

        for pattern_input in &collected_patterns {
            match pattern_repo.create(pattern_input, 1) {
                Ok(_) => {
                    final_result.patterns_created += 1;
                }
                Err(e) => {
                    final_result
                        .errors
                        .push(format!("Failed to create pattern: {}", e));
                }
            }
        }
    }

    // Mark rejections as processed if patterns were created
    if final_result.patterns_created > 0 || final_result.patterns_updated > 0 {
        let db_guard = db.lock().map_err(|e| anyhow::anyhow!("db lock: {}", e))?;
        let rejection_repo = db_guard.rejection_repo();
        rejection_repo.mark_processed(&rejection_ids)?;

        final_result.rejections_processed = rejections_count;

        // Update last compaction time
        let state_repo = db_guard.learning_state_repo();
        state_repo.set("last_compaction_at", &chrono::Utc::now().to_rfc3339())?;
    }

    Ok(final_result)
}

/// Build the prompt for the learning agent.
fn build_learning_prompt(
    rejections: &[FeedbackRejection],
    existing_patterns: &[LearnedPattern],
) -> Result<String> {
    #[derive(serde::Serialize)]
    struct RejectionItem {
        title: String,
        impact: String,
        confidence: f64,
        file_extension: Option<String>,
        agent_id: String,
    }

    #[derive(serde::Serialize)]
    struct PatternItem {
        id: String,
        pattern_text: String,
        category: Option<String>,
        file_extension: Option<String>,
        source_count: i32,
    }

    let rejection_items: Vec<RejectionItem> = rejections
        .iter()
        .map(|r| RejectionItem {
            title: r.title.clone(),
            impact: r.impact.clone(),
            confidence: r.confidence,
            file_extension: r.file_extension.clone(),
            agent_id: r.agent_id.clone(),
        })
        .collect();

    let pattern_items: Vec<PatternItem> = existing_patterns
        .iter()
        .map(|p| PatternItem {
            id: p.id.clone(),
            pattern_text: p.pattern_text.clone(),
            category: p.category.clone(),
            file_extension: p.file_extension.clone(),
            source_count: p.source_count,
        })
        .collect();

    prompts::render(
        "compact_learnings",
        &json!({
            "rejections": rejection_items,
            "existing_patterns": if pattern_items.is_empty() { None } else { Some(pattern_items) },
        }),
    )
    .context("failed to render compact_learnings prompt")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_learning_prompt_empty() {
        let result = build_learning_prompt(&[], &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_learning_prompt_with_rejections() {
        let rejections = vec![FeedbackRejection {
            id: "rej-1".to_string(),
            feedback_id: "fb-1".to_string(),
            review_id: "rev-1".to_string(),
            rule_id: None,
            title: "Unwrap usage".to_string(),
            impact: "nitpick".to_string(),
            confidence: 0.8,
            file_extension: Some("rs".to_string()),
            agent_id: "agent-1".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        }];

        let result = build_learning_prompt(&rejections, &[]);
        assert!(result.is_ok());
        let prompt = result.unwrap();
        assert!(prompt.contains("Unwrap usage"));
    }
}
