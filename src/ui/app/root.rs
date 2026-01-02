//! Root egui app struct.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::mpsc;

use crate::infra::db::repository::RepoRepository;
use crate::infra::db::{
    CommentRepository, Database, FeedbackLinkRepository, FeedbackRepository, ReviewRepository,
    ReviewRunRepository, TaskRepository,
};

use super::messages::{GenMsg, GhMsg};
use super::state::AppState;

/// Root egui application for LaReview.
pub struct LaReviewApp {
    pub state: AppState,

    pub task_repo: Arc<TaskRepository>,
    pub feedback_repo: Arc<FeedbackRepository>,
    pub feedback_link_repo: Arc<FeedbackLinkRepository>,
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

impl LaReviewApp {
    fn create_linked_repo(path: PathBuf) -> crate::domain::LinkedRepo {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let remotes = crate::infra::vcs::git::extract_git_remotes(&path);

        crate::domain::LinkedRepo {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            path,
            remotes,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Link a repository asynchronously - used for background linking
    pub fn link_repo(&mut self, path: PathBuf) -> Result<(), String> {
        if self
            .state
            .domain
            .linked_repos
            .iter()
            .any(|r| r.path == path)
        {
            return Ok(());
        }

        let repo = Self::create_linked_repo(path);

        let repo_repo = self.repo_repo.clone();
        let action_tx = self.action_tx.clone();

        tokio::spawn(async move {
            if let Err(e) = repo_repo.save(&repo) {
                eprintln!("Failed to save linked repo: {e}");
                return;
            }

            let _ = action_tx
                .send(crate::ui::app::Action::Async(
                    crate::ui::app::AsyncAction::RepoSaved(Ok(repo)),
                ))
                .await;
        });

        Ok(())
    }

    /// Link a repository synchronously - used for CLI handoff
    pub fn link_repo_sync(&mut self, path: PathBuf) -> Result<crate::domain::LinkedRepo, String> {
        if let Some(existing) = self
            .state
            .domain
            .linked_repos
            .iter()
            .find(|r| r.path == path)
        {
            return Ok(existing.clone());
        }

        let repo = Self::create_linked_repo(path);

        let rt = crate::runtime();
        rt.block_on(async {
            self.repo_repo.save(&repo).map_err(|e| e.to_string())?;
            Ok(repo)
        })
    }
}
