use super::client::LaReviewClient;
use super::prompt::{build_client_capabilities, build_prompt};
use super::validation::validate_tasks_payload;
use agent_client_protocol::{
    Agent, ClientSideConnection, ContentBlock, Implementation, InitializeRequest, McpServer,
    McpServerStdio, NewSessionRequest, PromptRequest, ProtocolVersion, TextContent,
};
use anyhow::{Context as _, Result};
use futures::future::LocalBoxFuture;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::runtime::Builder;
use tokio::task::LocalSet;

use super::{GenerateTasksInput, GenerateTasksResult, ProgressEvent};

fn push_log(logs: &Arc<Mutex<Vec<String>>>, message: impl Into<String>, debug: bool) {
    let msg = message.into();
    if let Ok(mut guard) = logs.lock() {
        guard.push(msg.clone());
    }
    if debug {
        eprintln!("[acp] {msg}");
    }
}

/// Resolve the binary to launch for the MCP task server, preferring explicit overrides.
pub(super) fn resolve_task_mcp_server_path(
    override_path: Option<&PathBuf>,
    current_exe: &Path,
) -> PathBuf {
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
                    let cancel_token = input.cancel_token.clone();
                    tokio::select! {
                        res = tokio::time::timeout(
                            Duration::from_secs(timeout_secs),
                            generate_tasks_with_acp_inner(input),
                        ) => {
                            res.map_err(|_| {
                                anyhow::anyhow!(format!("Agent timed out after {timeout_secs}s"))
                            })?
                        }
                        _ = async {
                            if let Some(token) = cancel_token {
                                token.cancelled().await;
                            } else {
                                std::future::pending::<()>().await;
                            }
                        } => {
                            Err(anyhow::anyhow!("Agent generation cancelled by user"))
                        }
                    }
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
        run_context,
        repo_root,
        agent_command,
        agent_args,
        progress_tx,
        mcp_server_binary,
        timeout_secs: _,
        cancel_token,
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

    if let Some(root) = &repo_root {
        log_fn(format!("repo access: enabled ({})", root.display()));
    } else {
        log_fn("repo access: disabled (diff-only)".to_string());
    }

    log_fn(format!("spawn: {} {}", agent_command, agent_args.join(" ")));

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

    // Spawn a background task to monitor the agent's stderr. Logs are forwarded
    // to the UI's progress stream to aid in debugging agent-side issues.
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
        run_context.run_id.clone(),
        repo_root.clone(),
    );
    let messages = client.messages.clone();
    let thoughts = client.thoughts.clone();
    let tasks_capture = client.tasks.clone();
    let raw_tasks_capture = client.raw_tasks_payload.clone();
    let finalization_received_capture = client.finalization_received.clone(); // Add this to check finalization later

    // Convert tokio streams for ACP
    use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
    let stdin_compat = stdin.compat_write();
    let stdout_compat = stdout.compat();

    let spawn_fn = |fut: LocalBoxFuture<'static, ()>| {
        tokio::task::spawn_local(fut);
    };

    let (connection, io_future) =
        ClientSideConnection::new(client, stdin_compat, stdout_compat, spawn_fn);

    // Spawn the asynchronous I/O loop to facilitate bidirectional
    // communication with the agent process.
    let io_handle = tokio::task::spawn_local(async move {
        let _ = io_future.await;
    });

    // Initialize connection
    push_log(&logs, "initialize", debug);
    connection
        .initialize(
            InitializeRequest::new(ProtocolVersion::V1)
                .client_info(Implementation::new("lareview", env!("CARGO_PKG_VERSION")))
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
    let cwd: PathBuf = match &repo_root {
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
        tempfile::NamedTempFile::new().context("create run context file for MCP server")?;
    std::fs::write(
        pr_context_file.path(),
        serde_json::to_string(&run_context).context("serialize run context")?,
    )
    .context("write run context file")?;

    let mut mcp_args = vec![
        "--task-mcp-server".to_string(),
        "--pr-context".to_string(),
        pr_context_file.path().to_string_lossy().to_string(),
    ];
    if let Ok(db_path) = std::env::var("LAREVIEW_DB_PATH") {
        mcp_args.push("--db-path".to_string());
        mcp_args.push(db_path);
    }
    if let Some(root) = &repo_root {
        mcp_args.push("--repo-root".to_string());
        mcp_args.push(root.to_string_lossy().to_string());
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
    let prompt_text = build_prompt(&run_context, repo_root.as_ref())?;
    push_log(&logs, "prompt", debug);
    let prompt_result = connection
        .prompt(PromptRequest::new(
            session.session_id,
            vec![ContentBlock::Text(TextContent::new(prompt_text))],
        ))
        .await;

    if let Err(err) = &prompt_result {
        push_log(
            &logs,
            format!("prompt error: {err:?}"),
            /* always log to stderr when debug */ true,
        );
        if let Ok(Some(status)) = child.try_wait() {
            push_log(
                &logs,
                format!("agent exited before prompt completed: {status}"),
                true,
            );
        }

        // If finalization was already received, the agent successfully completed its work
        // even if the prompt() call itself failed (e.g., due to early termination).
        // We only fail on prompt errors if finalization was NOT received.
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

    // Monitor the agent's execution until it terminates or is cancelled.
    // If the agent signals completion via `finalize_review`, it is terminated
    // after a short grace period to prevent UI hangs from lingering processes.
    let status = loop {
        let finalization_received = *finalization_received_capture.lock().unwrap();

        if finalization_received {
            push_log(
                &logs,
                "finalization received; terminating agent immediately",
                debug,
            );

            // Immediately kill the process. We don't wait for graceful exit because
            // the agent has explicitly signaled it is done.
            let _ = child.start_kill();
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
            return Err(anyhow::anyhow!("Agent generation cancelled by user"));
        }

        tokio::select! {
            res = child.wait() => {
                break res?;
            }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                // Check if agent exited with an error but we didn't get any tasks
                if child.try_wait().unwrap_or(None).is_some() {
                    break child.wait().await?;
                }
            }
        }
    };
    // Ensure all pending notifications from the agent are processed before return.
    let _ = io_handle.await;

    push_log(&logs, format!("Agent exit status: {}", status), debug);

    // Get captured tasks (now stored as Vec<ReviewTask> instead of Option<Vec<ReviewTask>>)
    let final_tasks = tasks_capture.lock().unwrap().clone();
    let finalization_received = *finalization_received_capture.lock().unwrap();

    // Collect output for return and for richer error contexts
    let final_messages = messages.lock().unwrap().clone();
    let final_thoughts = thoughts.lock().unwrap().clone();

    if final_tasks.is_empty() || !finalization_received {
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

        let error_msg = if final_tasks.is_empty() && !finalization_received {
            "Agent completed but did not call MCP tools return_task (no tasks captured) or finalize_review (no finalization)"
        } else if final_tasks.is_empty() {
            "Agent completed but did not call MCP tool return_task (no tasks captured)"
        } else {
            "Agent completed but did not call MCP tool finalize_review (no finalization)"
        };

        return Err(anyhow::anyhow!(error_msg).context(ctx_parts.join("\n\n")));
    }

    let raw_payload = raw_tasks_capture.lock().unwrap().clone();
    let warnings =
        validate_tasks_payload(&final_tasks, raw_payload.as_ref(), &run_context.diff_text)?;
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
mod tests {
    use super::*;

    #[test]
    fn test_resolve_task_mcp_server_path() {
        let current = PathBuf::from("/bin/lareview");
        let override_p = PathBuf::from("/custom/path");

        assert_eq!(
            resolve_task_mcp_server_path(Some(&override_p), &current),
            override_p
        );
        // We can't easily test option_env without changing build environment,
        // but we can test the fallback to current_exe.
        assert_eq!(resolve_task_mcp_server_path(None, &current), current);
    }

    #[test]
    fn test_push_log() {
        let logs = Arc::new(Mutex::new(Vec::new()));
        push_log(&logs, "test log", false);
        assert_eq!(logs.lock().unwrap().len(), 1);
        assert_eq!(logs.lock().unwrap()[0], "test log");
    }

    #[tokio::test]
    async fn test_generate_tasks_with_acp_invalid_command() {
        use crate::infra::acp::RunContext;
        let input = GenerateTasksInput {
            run_context: RunContext {
                review_id: "r".into(),
                run_id: "run".into(),
                agent_id: "a".into(),
                input_ref: "ref".into(),
                diff_text: "diff".into(),
                diff_hash: "h".into(),
                source: crate::domain::ReviewSource::DiffPaste {
                    diff_hash: "h".into(),
                },
                initial_title: None,
                created_at: None,
            },
            repo_root: None,
            agent_command: "non_existent_binary".into(),
            agent_args: vec![],
            progress_tx: None,
            mcp_server_binary: None,
            timeout_secs: Some(1),
            cancel_token: None,
            debug: false,
        };

        let result = generate_tasks_with_acp(input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_tasks_with_acp_timeout() {
        use crate::infra::acp::RunContext;
        let input = GenerateTasksInput {
            run_context: RunContext {
                review_id: "r".into(),
                run_id: "run".into(),
                agent_id: "a".into(),
                input_ref: "ref".into(),
                diff_text: "diff".into(),
                diff_hash: "h".into(),
                source: crate::domain::ReviewSource::DiffPaste {
                    diff_hash: "h".into(),
                },
                initial_title: None,
                created_at: None,
            },
            repo_root: None,
            agent_command: "sleep".into(),
            agent_args: vec!["10".into()],
            progress_tx: None,
            mcp_server_binary: None,
            timeout_secs: Some(1),
            cancel_token: None,
            debug: false,
        };

        let result = generate_tasks_with_acp(input).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn test_generate_tasks_with_acp_cancellation() {
        use crate::infra::acp::RunContext;
        use tokio_util::sync::CancellationToken;

        let token = CancellationToken::new();
        let token_clone = token.clone();

        let input = GenerateTasksInput {
            run_context: RunContext {
                review_id: "r".into(),
                run_id: "run".into(),
                agent_id: "a".into(),
                input_ref: "ref".into(),
                diff_text: "diff".into(),
                diff_hash: "h".into(),
                source: crate::domain::ReviewSource::DiffPaste {
                    diff_hash: "h".into(),
                },
                initial_title: None,
                created_at: None,
            },
            repo_root: None,
            agent_command: "sleep".into(),
            agent_args: vec!["10".into()],
            progress_tx: None,
            mcp_server_binary: None,
            timeout_secs: Some(10),
            cancel_token: Some(token_clone),
            debug: false,
        };

        // Cancel the token after a short delay
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            token.cancel();
        });

        let result = generate_tasks_with_acp(input).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("cancelled by user")
        );
    }
}
