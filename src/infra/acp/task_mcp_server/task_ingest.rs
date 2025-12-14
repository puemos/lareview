use super::config::ServerConfig;
use super::logging;
use super::parsing::parse_task;
use super::run_context::RunContext;
use crate::domain::ReviewTask;
use crate::infra::db::{Database, ReviewRepository, TaskRepository};
use crate::infra::diff_index::DiffIndex;
use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::Value;

pub(super) fn save_task(config: &ServerConfig, raw_task: Value) -> Result<ReviewTask> {
    let ctx = load_run_context(config);

    let db = match &config.db_path {
        Some(path) => Database::open_at(path.clone()),
        None => Database::open(),
    }
    .context("open database")?;
    let conn = db.connection();
    let task_repo = TaskRepository::new(conn);

    let mut task = parse_task(raw_task)?;
    task.run_id = ctx.run_id.clone();

    // Recalculate stats from diff_refs using the canonical diff from the run context
    if !task.diff_refs.is_empty() {
        // Always set files from the provided diff_refs
        let mut files = Vec::new();
        for diff_ref in &task.diff_refs {
            if !files.contains(&diff_ref.file) {
                files.push(diff_ref.file.clone());
            }
        }
        task.files = files;

        // Best-effort stats calculation. If the agent's hunk coordinates don't line up
        // with the canonical diff, log and fall back to the provided stats instead of
        // failing the whole tool call.
        match DiffIndex::new(&ctx.diff_text) {
            Ok(diff_index) => match diff_index.task_stats(&task.diff_refs) {
                Ok((additions, deletions)) => {
                    task.stats.additions = additions;
                    task.stats.deletions = deletions;
                }
                Err(err) => logging::log_to_file(
                    config,
                    &format!(
                        "return_task: diff_refs mismatch; using agent-provided stats. Error: {err}"
                    ),
                ),
            },
            Err(err) => logging::log_to_file(
                config,
                &format!(
                    "return_task: failed to build DiffIndex; using agent-provided stats. Error: {err}"
                ),
            ),
        }

    }

    task_repo
        .save(&task)
        .with_context(|| format!("save task {}", task.id))?;

    Ok(task)
}

pub(super) fn update_review_metadata(config: &ServerConfig, args: Value) -> Result<()> {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let summary = args
        .get("summary")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let db = match &config.db_path {
        Some(path) => Database::open_at(path.clone()),
        None => Database::open(),
    }
    .context("open database")?;
    let conn = db.connection();
    let review_repo = ReviewRepository::new(conn);

    if let Some(title) = title {
        // Note: run_context_id is not available in ServerConfig, so we'll use ctx.review_id from load_run_context
        let ctx = load_run_context(config);
        review_repo.update_title_and_summary(&ctx.review_id, &title, summary.as_deref())
            .context("update review title and summary")?;
    }

    Ok(())
}

fn load_run_context(config: &ServerConfig) -> RunContext {
    if let Some(path) = &config.run_context
        && let Ok(content) = std::fs::read_to_string(path)
            && let Ok(ctx) = serde_json::from_str::<RunContext>(&content) {
                return ctx;
            }

    RunContext {
        review_id: "local-review".to_string(),
        run_id: "local-run".to_string(),
        agent_id: "unknown".to_string(),
        input_ref: "unknown".to_string(),
        diff_text: String::new(),
        diff_hash: String::new(),
        source: crate::domain::ReviewSource::DiffPaste {
            diff_hash: String::new(),
        },
        initial_title: Some("Review".to_string()),
        created_at: Some(Utc::now().to_rfc3339()),
    }
}
