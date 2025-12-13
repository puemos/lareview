use super::config::ServerConfig;
use super::logging::log_to_file;
use super::persistence::persist_review_to_db;
use pmcp::{SimpleTool, ToolHandler};
use serde_json::{Value, json};
use std::sync::Arc;

/// Create the return_tasks tool with proper description and schema.
pub(super) fn create_return_tasks_tool(config: Arc<ServerConfig>) -> impl ToolHandler {
    SimpleTool::new("return_tasks", move |args: Value, _extra| {
        let config = config.clone();
        Box::pin(async move {
            log_to_file(&config, "return_tasks called");
            let persist_args = args.clone();
            let persist_args_for_log = persist_args.clone();

            let persist_config = config.clone();
            let persist_result = tokio::task::spawn_blocking(move || {
                persist_review_to_db(&persist_config, persist_args)
            })
            .await;

            match persist_result {
                Ok(Ok(())) => log_to_file(
                    &config,
                    &format!("ReturnTasksTool persisted tasks to DB: {persist_args_for_log}"),
                ),
                Ok(Err(err)) => {
                    log_to_file(
                        &config,
                        &format!("ReturnTasksTool failed to persist tasks: {err}"),
                    );
                    return Err(pmcp::Error::Validation(format!(
                        "invalid return_tasks payload: {err}"
                    )));
                }
                Err(join_err) => {
                    log_to_file(
                        &config,
                        &format!("ReturnTasksTool task join error: {join_err}"),
                    );
                    return Err(pmcp::Error::Internal(format!(
                        "return_tasks persistence join error: {join_err}"
                    )));
                }
            }

            if let Some(path) = &config.tasks_out {
                log_to_file(
                    &config,
                    &format!("ReturnTasksTool writing to {}", path.display()),
                );
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
         id, title, description, stats (risk, tags), and diffs. \
         The server computes files and line additions/deletions from the provided diffs. \
         Optionally include sub_flow (grouping name) and diagram (D2 format for visualization).",
    )
    .with_schema(task_schema())
}

/// Create the return_review tool with proper description and schema.
pub(super) fn create_return_review_tool(config: Arc<ServerConfig>) -> impl ToolHandler {
    SimpleTool::new("return_review", move |args: Value, _extra| {
        let config = config.clone();
        Box::pin(async move {
            log_to_file(&config, "return_review called");
            let persist_args = args.clone();
            let persist_args_for_log = persist_args.clone();

            let persist_config = config.clone();
            let persist_result = tokio::task::spawn_blocking(move || {
                persist_review_to_db(&persist_config, persist_args)
            })
            .await;

            match persist_result {
                Ok(Ok(())) => log_to_file(
                    &config,
                    &format!(
                        "ReturnReviewTool persisted review+tasks to DB: {persist_args_for_log}"
                    ),
                ),
                Ok(Err(err)) => {
                    log_to_file(
                        &config,
                        &format!("ReturnReviewTool failed to persist review+tasks: {err}"),
                    );
                    return Err(pmcp::Error::Validation(format!(
                        "invalid return_review payload: {err}"
                    )));
                }
                Err(join_err) => {
                    log_to_file(
                        &config,
                        &format!("ReturnReviewTool task join error: {join_err}"),
                    );
                    return Err(pmcp::Error::Internal(format!(
                        "return_review persistence join error: {join_err}"
                    )));
                }
            }

            if let Some(path) = &config.tasks_out {
                log_to_file(
                    &config,
                    &format!("ReturnReviewTool writing to {}", path.display()),
                );
                let _ = std::fs::write(path, args.to_string());
                log_to_file(&config, "ReturnReviewTool write complete");
            }

            Ok(json!({ "status": "ok", "message": "Review received successfully" }))
        })
    })
    .with_description(
        "Submit the complete review output: an agent-generated review title/summary plus a set \
         of intent-driven review tasks that together cover 100% of the provided diff. Call this \
         tool once at the end of your analysis.",
    )
    .with_schema(review_schema())
}

fn review_schema() -> Value {
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
            },
            "tasks": task_schema()["properties"]["tasks"].clone()
        },
        "required": ["title", "tasks"]
    })
}

fn task_schema() -> Value {
    json!({
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
                        "stats": {
                            "type": "object",
                            "description": "Risk and tags for this task. Additions, deletions, and files are computed from diffs.",
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
                            "required": ["risk", "tags"]
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
                    "required": ["id", "title", "description", "stats", "diffs"]
                }
            }
        },
        "required": ["tasks"]
    })
}
