use super::config::ServerConfig;
use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::Value;

use crate::domain::PullRequest;
use crate::infra::db::{Database, PullRequestRepository, TaskRepository};

pub(super) fn persist_tasks_to_db(config: &ServerConfig, args: Value) -> Result<()> {
    let tasks = super::parse_tasks(args)?;
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
        task.pr_id = pull_request.id.clone();
        task_repo
            .save(&task)
            .with_context(|| format!("save task {}", task.id))?;
    }

    Ok(())
}

#[allow(clippy::collapsible_if)]
fn load_pull_request(config: &ServerConfig) -> PullRequest {
    if let Some(path) = &config.pr_context {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(pr) = serde_json::from_str::<PullRequest>(&content) {
                return pr;
            }
        }
    }

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
