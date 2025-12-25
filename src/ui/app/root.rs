//! Root egui app struct.

use std::sync::Arc;

use tokio::sync::mpsc;

use crate::infra::db::repository::RepoRepository;
use crate::infra::db::{
    CommentRepository, Database, ReviewRepository, ReviewRunRepository, TaskRepository,
    ThreadRepository,
};

use super::messages::{GenMsg, GhMsg};
use super::state::AppState;

/// Root egui application for LaReview.
pub struct LaReviewApp {
    pub state: AppState,

    pub task_repo: Arc<TaskRepository>,
    pub thread_repo: Arc<ThreadRepository>,
    pub comment_repo: Arc<CommentRepository>,
    pub review_repo: Arc<ReviewRepository>,
    pub run_repo: Arc<ReviewRunRepository>,
    pub repo_repo: Arc<RepoRepository>,

    pub _db: Database,

    pub gen_tx: mpsc::Sender<GenMsg>,
    pub gen_rx: mpsc::Receiver<GenMsg>,

    pub gh_tx: mpsc::Sender<GhMsg>,
    pub gh_rx: mpsc::Receiver<GhMsg>,

    pub d2_install_tx: mpsc::Sender<String>,
    pub d2_install_rx: mpsc::Receiver<String>,

    pub action_tx: mpsc::Sender<crate::ui::app::Action>,
    pub action_rx: mpsc::Receiver<crate::ui::app::Action>,

    pub agent_task: Option<tokio::task::JoinHandle<()>>,
    pub agent_cancel_token: Option<tokio_util::sync::CancellationToken>,

    pub skip_runtime: bool,
}
