use super::config::ServerConfig;
use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::Value;

use crate::domain::{Review, ReviewRun, ReviewSource};
use crate::infra::db::{Database, ReviewRepository, ReviewRunRepository, TaskRepository};

use super::RunContext;

pub(super) fn persist_review_to_db(config: &ServerConfig, args: Value) -> Result<()> {
    let tasks = super::parse_tasks(args.clone())?;
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let summary = args
        .get("summary")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let ctx = load_run_context(config);

    let db = match &config.db_path {
        Some(path) => Database::open_at(path.clone()),
        None => Database::open(),
    }
    .context("open database")?;
    let conn = db.connection();
    let review_repo = ReviewRepository::new(conn.clone());
    let run_repo = ReviewRunRepository::new(conn.clone());
    let task_repo = TaskRepository::new(conn);

    let now = Utc::now().to_rfc3339();
    let created_at = ctx.created_at.clone().unwrap_or_else(|| now.clone());

    let review_title = title
        .clone()
        .or(ctx.initial_title.clone())
        .unwrap_or_else(|| "Review".to_string());

    let review = Review {
        id: ctx.review_id.clone(),
        title: review_title,
        summary,
        source: ctx.source.clone(),
        active_run_id: Some(ctx.run_id.clone()),
        created_at: created_at.clone(),
        updated_at: now.clone(),
    };

    let run = ReviewRun {
        id: ctx.run_id.clone(),
        review_id: ctx.review_id.clone(),
        agent_id: ctx.agent_id.clone(),
        input_ref: ctx.input_ref.clone(),
        diff_text: ctx.diff_text.clone(),
        diff_hash: ctx.diff_hash.clone(),
        created_at,
    };

    review_repo.save(&review).context("save review")?;
    run_repo.save(&run).context("save review run")?;

    for mut task in tasks {
        task.run_id = ctx.run_id.clone();
        task_repo
            .save(&task)
            .with_context(|| format!("save task {}", task.id))?;
    }

    Ok(())
}

#[allow(clippy::collapsible_if)]
fn load_run_context(config: &ServerConfig) -> RunContext {
    if let Some(path) = &config.run_context {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(ctx) = serde_json::from_str::<RunContext>(&content) {
                return ctx;
            }
        }
    }

    RunContext {
        review_id: "local-review".to_string(),
        run_id: "local-run".to_string(),
        agent_id: "unknown".to_string(),
        input_ref: "unknown".to_string(),
        diff_text: String::new(),
        diff_hash: String::new(),
        source: ReviewSource::DiffPaste {
            diff_hash: String::new(),
        },
        initial_title: Some("Review".to_string()),
        created_at: Some(Utc::now().to_rfc3339()),
    }
}
