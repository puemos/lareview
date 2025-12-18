//! Root egui app struct.

use std::sync::Arc;

use tokio::sync::mpsc;

use crate::infra::db::{
    Database, NoteRepository, ReviewRepository, ReviewRunRepository, TaskRepository,
};

use super::messages::{GenMsg, GhMsg};
use super::state::AppState;

/// Root egui application for LaReview.
pub struct LaReviewApp {
    pub state: AppState,

    pub task_repo: Arc<TaskRepository>,
    pub note_repo: Arc<NoteRepository>,
    pub review_repo: Arc<ReviewRepository>,
    pub run_repo: Arc<ReviewRunRepository>,

    pub _db: Database,

    pub gen_tx: mpsc::Sender<GenMsg>,
    pub gen_rx: mpsc::Receiver<GenMsg>,

    pub gh_tx: mpsc::Sender<GhMsg>,
    pub gh_rx: mpsc::Receiver<GhMsg>,

    pub d2_install_tx: mpsc::Sender<String>,
    pub d2_install_rx: mpsc::Receiver<String>,

    pub action_tx: mpsc::Sender<crate::ui::app::Action>,
    pub action_rx: mpsc::Receiver<crate::ui::app::Action>,
}
