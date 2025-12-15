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

/// Sanitize the `diff_refs` in a raw task JSON value. This function iterates
/// through the hunks in each diff_ref and removes any that are not valid objects
/// with all required fields. This prevents deserialization errors later on.
fn clean_raw_task_hunks(raw_task: &mut Value, config: &ServerConfig) {
    if let Some(diff_refs) = raw_task.get_mut("diff_refs").and_then(|v| v.as_array_mut()) {
        for diff_ref in diff_refs.iter_mut() {
            if let Some(hunks) = diff_ref.get_mut("hunks").and_then(|v| v.as_array_mut()) {
                hunks.retain(|hunk| {
                    if let Some(obj) = hunk.as_object() {
                        let is_valid = obj.contains_key("old_start")
                            && obj.contains_key("old_lines")
                            && obj.contains_key("new_start")
                            && obj.contains_key("new_lines");
                        if !is_valid {
                            logging::log_to_file(
                                config,
                                &format!("clean_raw_task_hunks: removing incomplete hunk object: {hunk:?}"),
                            );
                        }
                        is_valid
                    } else {
                        // Not an object, remove it
                        logging::log_to_file(
                            config,
                            &format!("clean_raw_task_hunks: removing non-object hunk: {hunk:?}"),
                        );
                        false
                    }
                });
            }
        }
    }
}

pub(super) fn save_task(config: &ServerConfig, raw_task: Value) -> Result<ReviewTask> {
    let ctx = load_run_context(config);

    // Sanitize the raw task before parsing to avoid deserialization errors
    // due to potentially malformed hunk objects from the agent.
    let mut cleaned_raw_task = raw_task;
    clean_raw_task_hunks(&mut cleaned_raw_task, config);

    let db = Database::open().context("open database")?;
    let conn = db.connection();
    let task_repo = TaskRepository::new(conn.clone());
    let review_run_repo = crate::infra::db::ReviewRunRepository::new(conn.clone());

    // Ensure the review run exists in the database before saving the task
    let review_run = crate::domain::ReviewRun {
        id: ctx.run_id.clone(),
        review_id: ctx.review_id.clone(),
        agent_id: ctx.agent_id.clone(),
        input_ref: ctx.input_ref.clone(),
        diff_text: ctx.diff_text.clone(),
        diff_hash: ctx.diff_hash.clone(),
        created_at: ctx
            .created_at
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
    };

    // Also ensure the parent review exists. `save` is non-destructive.
    let review_repo = crate::infra::db::ReviewRepository::new(conn.clone());
    let review = crate::domain::Review {
        id: ctx.review_id.clone(),
        title: ctx
            .initial_title
            .clone()
            .unwrap_or_else(|| "Untitled Review".to_string()),
        summary: None, // Always start with no summary; it's added by finalize_review
        source: ctx.source.clone(),
        active_run_id: Some(ctx.run_id.clone()),
        created_at: ctx
            .created_at
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    review_repo
        .save(&review)
        .with_context(|| format!("save review {}", ctx.review_id))?;

    // Always update the active run and updated_at timestamp
    review_repo
        .set_active_run(&ctx.review_id, &ctx.run_id)
        .with_context(|| format!("set active run for review {}", ctx.review_id))?;

    review_run_repo
        .save(&review_run)
        .with_context(|| format!("save review run {}", ctx.run_id))?;

    let mut task = parse_task(cleaned_raw_task.clone())?;
    task.run_id = ctx.run_id.clone();

    if !task.diff_refs.is_empty() {
        // Always set files from the provided diff_refs
        let mut files = Vec::new();
        for diff_ref in &task.diff_refs {
            if !files.contains(&diff_ref.file) {
                files.push(diff_ref.file.clone());
            }
        }
        task.files = files;

        // Best-effort stats calculation.
        match DiffIndex::new(&ctx.diff_text) {
            Ok(diff_index) => match diff_index.task_stats(&task.diff_refs) {
                Ok((additions, deletions)) => {
                    task.stats.additions = additions;
                    task.stats.deletions = deletions;
                }
                Err(err) => {
                    logging::log_to_file(
                        config,
                        &format!(
                            "return_task: diff_refs mismatch; using agent-provided stats. Error: {err:?}"
                        ),
                    );
                }
            },
            Err(err) => {
                logging::log_to_file(
                    config,
                    &format!(
                        "return_task: failed to build DiffIndex; using agent-provided stats. Error: {err:?}"
                    ),
                );
            }
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

    if title.is_none() && summary.is_none() {
        return Ok(());
    }

    let ctx = load_run_context(config);
    let db = Database::open().context("open database")?;
    let conn = db.connection();
    let review_repo = ReviewRepository::new(conn.clone());
    let review_run_repo = crate::infra::db::ReviewRunRepository::new(conn.clone());

    // Ensure the review exists by saving a placeholder. `save` is non-destructive.
    let review_placeholder = crate::domain::Review {
        id: ctx.review_id.clone(),
        title: ctx
            .initial_title
            .clone()
            .unwrap_or_else(|| "Untitled Review".to_string()),
        summary: None,
        source: ctx.source.clone(),
        active_run_id: Some(ctx.run_id.clone()),
        created_at: ctx
            .created_at
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    review_repo.save(&review_placeholder)?;

    // Fetch the review's current title if a new one isn't provided
    let review_title = if let Some(t) = title {
        t
    } else {
        review_repo
            .find_by_id(&ctx.review_id)?
            .map(|r| r.title)
            .unwrap_or(review_placeholder.title)
    };

    // Update the review with the new metadata
    review_repo
        .set_active_run(&ctx.review_id, &ctx.run_id)
        .with_context(|| format!("set active run for review {}", ctx.review_id))?;
    review_repo
        .update_title_and_summary(&ctx.review_id, &review_title, summary.as_deref())
        .context("update review title and summary")?;

    let review_run = crate::domain::ReviewRun {
        id: ctx.run_id.clone(),
        review_id: ctx.review_id.clone(),
        agent_id: ctx.agent_id.clone(),
        input_ref: ctx.input_ref.clone(),
        diff_text: ctx.diff_text.clone(),
        diff_hash: ctx.diff_hash.clone(),
        created_at: ctx
            .created_at
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
    };
    review_run_repo
        .save(&review_run)
        .with_context(|| format!("save review run {}", ctx.run_id))?;

    Ok(())
}

pub(super) fn load_run_context(config: &ServerConfig) -> RunContext {
    if let Some(path) = &config.run_context
        && let Ok(content) = std::fs::read_to_string(path)
        && let Ok(ctx) = serde_json::from_str::<RunContext>(&content)
    {
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
