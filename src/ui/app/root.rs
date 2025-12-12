//! Root egui app struct.

use std::sync::Arc;

use tokio::sync::mpsc;

use crate::infra::db::{Database, NoteRepository, PullRequestRepository, TaskRepository};

use super::messages::GenMsg;
use super::state::AppState;

/// Root egui application for LaReview.
pub struct LaReviewApp {
    pub state: AppState,

    pub task_repo: Arc<TaskRepository>,
    pub note_repo: Arc<NoteRepository>,
    pub pr_repo: Arc<PullRequestRepository>,

    pub _db: Database,

    pub gen_tx: mpsc::Sender<GenMsg>,
    pub gen_rx: mpsc::Receiver<GenMsg>,

    pub d2_install_tx: mpsc::Sender<String>,
    pub d2_install_rx: mpsc::Receiver<String>,
}
