use super::config::ServerConfig;
use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::Value;

use crate::domain::{Review, ReviewRun, ReviewSource};
use crate::infra::db::{Database, ReviewRepository, ReviewRunRepository, TaskRepository};

use super::RunContext;


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
