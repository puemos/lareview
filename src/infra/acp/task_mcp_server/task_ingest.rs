use super::config::ServerConfig;
use super::parsing::parse_task;
use super::run_context::RunContext;
use crate::domain::ReviewTask;
use crate::infra::db::{Database, ReviewRepository, TaskRepository};
use crate::infra::diff_index::DiffIndex;
use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::Value;

/// Validate raw hunk objects before parsing so we fail fast on malformed payloads.
fn validate_raw_task_hunks(raw_task: &Value) -> Result<()> {
    let diff_refs = match raw_task.get("diff_refs").and_then(|v| v.as_array()) {
        Some(diff_refs) => diff_refs,
        None => return Ok(()),
    };

    for diff_ref in diff_refs {
        let hunks = match diff_ref.get("hunks") {
            Some(hunks) => hunks
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("diff_refs.hunks must be an array of objects"))?,
            None => continue,
        };

        for hunk in hunks {
            let obj = hunk
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("diff_refs.hunks entries must be objects"))?;
            let has_fields = obj.contains_key("old_start")
                && obj.contains_key("old_lines")
                && obj.contains_key("new_start")
                && obj.contains_key("new_lines");
            if !has_fields {
                anyhow::bail!(
                    "diff_refs.hunks entries must include old_start, old_lines, new_start, new_lines"
                );
            }
        }
    }

    Ok(())
}

fn validate_task_diff_refs(task: &ReviewTask, diff_index: &DiffIndex) -> Result<()> {
    if task.diff_refs.is_empty() {
        anyhow::bail!(
            "Task {} missing diff_refs. Use hunk_manifest_json to populate diff_refs.",
            task.id
        );
    }

    for diff_ref in &task.diff_refs {
        let file = diff_ref.file.as_str();
        if file.trim().is_empty() {
            anyhow::bail!(
                "Task {} has an empty diff_ref file. Use file paths from hunk_manifest_json.",
                task.id
            );
        }
        if file != file.trim() {
            anyhow::bail!(
                "Task {} has whitespace in diff_ref file '{}'. Copy file paths exactly from hunk_manifest_json.",
                task.id,
                file
            );
        }
        if file.starts_with("a/") || file.starts_with("b/") {
            anyhow::bail!(
                "Task {} diff_ref file '{}' must not include a/ or b/ prefixes. Use file paths from hunk_manifest_json.",
                task.id,
                file
            );
        }

        if diff_ref.hunks.is_empty() {
            diff_index.validate_file_exists(file)?;
            continue;
        }

        for hunk_ref in &diff_ref.hunks {
            diff_index.validate_hunk_exists(file, hunk_ref)?;
        }
    }

    Ok(())
}

fn validate_task_diagram(task: &ReviewTask) -> Result<()> {
    let has_diagram = task
        .diagram
        .as_ref()
        .is_some_and(|diagram| !diagram.trim().is_empty());
    if !has_diagram {
        anyhow::bail!(
            "Task {} missing D2 diagram. Every task must include a diagram.",
            task.id
        );
    }
    Ok(())
}

pub(super) fn open_database(config: &ServerConfig) -> Result<Database> {
    if let Some(path) = &config.db_path {
        Database::open_at(path.clone()).context("open database")
    } else {
        Database::open().context("open database")
    }
}

pub(super) fn save_task(config: &ServerConfig, raw_task: Value) -> Result<ReviewTask> {
    let ctx = load_run_context(config);

    // Fail fast on malformed hunks to avoid silently changing scope.
    validate_raw_task_hunks(&raw_task)?;

    let db = open_database(config)?;
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

    let mut task = parse_task(raw_task.clone())?;
    task.run_id = ctx.run_id.clone();

    let diff_index = DiffIndex::new(&ctx.diff_text)?;
    validate_task_diff_refs(&task, &diff_index)?;
    validate_task_diagram(&task)?;

    // Always set files from the provided diff_refs
    let mut files = Vec::new();
    for diff_ref in &task.diff_refs {
        if !files.contains(&diff_ref.file) {
            files.push(diff_ref.file.clone());
        }
    }
    task.files = files;

    let (additions, deletions) = diff_index.task_stats(&task.diff_refs)?;
    task.stats.additions = additions;
    task.stats.deletions = deletions;

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
    let db = open_database(config)?;
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
        diff_text: String::new().into(),
        diff_hash: String::new(),
        source: crate::domain::ReviewSource::DiffPaste {
            diff_hash: String::new(),
        },
        initial_title: Some("Review".to_string()),
        created_at: Some(Utc::now().to_rfc3339()),
    }
}
