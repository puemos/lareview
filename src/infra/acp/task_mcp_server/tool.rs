use super::config::ServerConfig;
use super::logging::log_to_file;
use super::task_ingest::{save_task, update_review_metadata};
use pmcp::{SimpleTool, ToolHandler};
use serde_json::{Value, json};
use std::sync::Arc;

/// Create the return_task tool for streaming task submission.
pub(super) fn create_return_task_tool(config: Arc<ServerConfig>) -> impl ToolHandler {
    SimpleTool::new("return_task", move |args: Value, _extra| {
        let config = config.clone();
        Box::pin(async move {
            log_to_file(&config, "return_task called");
            let raw_task = args.clone();

            let persist_config = config.clone();
            let persist_result = tokio::task::spawn_blocking(move || {
                save_task(&persist_config, raw_task)
            })
            .await;

            match persist_result {
                Ok(Ok(task)) => {
                    log_to_file(
                        &config,
                        &format!("ReturnTaskTool persisted task to DB: {}", task.id),
                    );

                    if let Some(path) = &config.tasks_out {
                        log_to_file(
                            &config,
                            &format!("ReturnTaskTool appending to {}", path.display()),
                        );

                        // Append the task as a JSON line to support streaming
                        let json_line = format!("{}\n", serde_json::to_string(&task).unwrap_or_default());
                        if let Err(e) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(path)
                            .and_then(|mut file| std::io::Write::write_all(&mut file, json_line.as_bytes()))
                        {
                            log_to_file(&config, &format!("Failed to write to tasks out file: {}", e));
                        }
                        log_to_file(&config, "ReturnTaskTool append complete");
                    }

                    Ok(json!({
                        "status": "ok",
                        "message": format!("Task {} received successfully", task.id),
                        "task_id": task.id
                    }))
                },
                Ok(Err(err)) => {
                    log_to_file(
                        &config,
                        &format!("ReturnTaskTool failed to persist task: {err}"),
                    );
                    Err(pmcp::Error::Validation(format!(
                        "invalid return_task payload: {err}"
                    )))
                }
                Err(join_err) => {
                    log_to_file(
                        &config,
                        &format!("ReturnTaskTool task join error: {join_err}"),
                    );
                    Err(pmcp::Error::Internal(format!(
                        "return_task persistence join error: {join_err}"
                    )))
                }
            }
        })
    })
    .with_description(
        "Submit a single code review task for a pull request. Call this repeatedly to submit each task individually. \
         Each task must include: id, title, description, stats (risk, tags), and diff_refs. \
         The server computes files and line additions/deletions from the provided diff_refs using the canonical diff. \
         Optionally include sub_flow (grouping name) and diagram (D2 format for visualization).",
    )
    .with_schema(single_task_schema())
}

/// Create the finalize_review tool for finalizing the review.
pub(super) fn create_finalize_review_tool(config: Arc<ServerConfig>) -> impl ToolHandler {
    SimpleTool::new("finalize_review", move |args: Value, _extra| {
        let config = config.clone();
        Box::pin(async move {
            log_to_file(&config, "finalize_review called");
            let persist_args = args.clone();
            let persist_args_for_log = persist_args.clone();

            let persist_config = config.clone();
            let persist_args_for_spawn = persist_args.clone(); // Clone before moving into spawn
            let persist_result = tokio::task::spawn_blocking(move || {
                update_review_metadata(&persist_config, persist_args_for_spawn)
            })
            .await;

            match persist_result {
                Ok(Ok(())) => {
                    log_to_file(
                        &config,
                        &format!("FinalizeReviewTool updated review metadata: {persist_args_for_log}"),
                    );

                    if let Some(path) = &config.tasks_out {
                        log_to_file(
                            &config,
                            &format!("FinalizeReviewTool writing metadata to {}", path.display()),
                        );

                        // Append the metadata as a final record
                        let metadata_record = json!({
                            "type": "review_metadata",
                            "title": persist_args_for_log.get("title").unwrap_or(&json!(null)),
                            "summary": persist_args_for_log.get("summary").unwrap_or(&json!(null))
                        });
                        let json_line = format!("{}\n", serde_json::to_string(&metadata_record).unwrap_or_default());
                        if let Err(e) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(path)
                            .and_then(|mut file| std::io::Write::write_all(&mut file, json_line.as_bytes()))
                        {
                            log_to_file(&config, &format!("Failed to write metadata to tasks out file: {}", e));
                        }
                        log_to_file(&config, "FinalizeReviewTool write complete");
                    }

                    Ok(json!({ "status": "ok", "message": "Review finalized successfully" }))
                },
                Ok(Err(err)) => {
                    log_to_file(
                        &config,
                        &format!("FinalizeReviewTool failed to update metadata: {err}"),
                    );
                    Err(pmcp::Error::Validation(format!(
                        "invalid finalize_review payload: {err}"
                    )))
                }
                Err(join_err) => {
                    log_to_file(
                        &config,
                        &format!("FinalizeReviewTool task join error: {join_err}"),
                    );
                    Err(pmcp::Error::Internal(format!(
                        "finalize_review persistence join error: {join_err}"
                    )))
                }
            }
        })
    })
    .with_description(
        "Finalize the review by submitting the agent-generated review title/summary. \
         Call this once at the end of your analysis after all tasks have been submitted via return_task.",
    )
    .with_schema(review_metadata_schema())
}

fn single_task_schema() -> Value {
    json!({
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
            "stats": {
                "type": "object",
                "properties": {
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
                "required": ["risk", "tags"],
                "description": "Risk and tags for this task. Additions, deletions, and files are computed from diff_refs."
            },
            "diff_refs": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "file": {
                            "type": "string",
                            "description": "File path in the diff (no a/ or b/ prefixes)"
                        },
                        "hunks": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "old_start": {"type": "integer"},
                                    "old_lines": {"type": "integer"},
                                    "new_start": {"type": "integer"},
                                    "new_lines": {"type": "integer"}
                                },
                                "required": ["old_start", "old_lines", "new_start", "new_lines"],
                                "description": "Hunk coordinates: (old_start, old_lines, new_start, new_lines)"
                            }
                        }
                    },
                    "required": ["file", "hunks"],
                    "description": "Reference to specific hunks in the diff by line numbers"
                },
                "description": "Array of references to specific hunks in the canonical diff. Each ref points to a specific file and range of lines."
            },
            "sub_flow": {
                "type": "string",
                "description": "Optional logical grouping name for this task. Use when multiple tasks belong to the same larger feature or concern (e.g., 'authentication-flow', 'data-migration', 'payment-processing'). Helps organize related tasks."
            },
            "diagram": {
                "type": "string",
                "description": "STRONGLY RECOMMENDED: D2 diagram visualizing the flow, sequence, architecture, or data model. Create diagrams for MEDIUM/HIGH risk tasks or when multiple components interact. Must be valid D2 syntax (e.g., 'Client -> API: Request\\nAPI -> DB: Query')"
            }
        },
        "required": ["id", "title", "description", "stats", "diff_refs"]
    })
}

fn review_metadata_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "title": {
                "type": "string",
                "description": "Agent-generated review title. For GitHub PRs, this may match or improve the PR title."
            },
            "summary": {
                "type": "string",
                "description": "Optional short executive summary of the change and primary risks."
            }
        },
        "required": ["title"]
    })
}
