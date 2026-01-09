use crate::infra::app_config::AppConfig;
use crate::infra::db::Database;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct DiffRequest {
    pub from: String,
    pub to: String,
    pub agent: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingDiff {
    pub diff: String,
    pub repo_root: Option<PathBuf>,
    pub agent: Option<String>,
    pub source: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub config: Arc<RwLock<AppConfig>>,
    pub diff_request: Arc<Mutex<Option<DiffRequest>>>,
    pub pending_diff: Arc<Mutex<Option<PendingDiff>>>,
    pub active_runs: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

impl AppState {
    pub fn new() -> Self {
        let db = Database::open().expect("Failed to open database");
        if let Err(err) = db.mark_stale_runs_failed() {
            log::warn!("Failed to mark stale runs as failed: {}", err);
        }
        Self {
            db: Arc::new(Mutex::new(db)),
            config: Arc::new(RwLock::new(AppConfig::default())),
            diff_request: Arc::new(Mutex::new(None)),
            pending_diff: Arc::new(Mutex::new(None)),
            active_runs: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
