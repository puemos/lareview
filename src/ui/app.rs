#![allow(dead_code)]

//! Main application state (egui version)

use std::sync::Arc;

use crate::acp::ProgressEvent;
use crate::data::db::Database;
use crate::data::repository::{NoteRepository, PullRequestRepository, TaskRepository};
use crate::domain::{Note, PullRequest, ReviewTask};

use eframe::egui;
use tokio::sync::mpsc;

/// Which screen is active
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppView {
    Generate,
    Review,
}

impl Default for AppView {
    fn default() -> Self {
        Self::Generate
    }
}

/// Which agent is selected (matches original code)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedAgent {
    Codex,
    Gemini,
}

impl Default for SelectedAgent {
    fn default() -> Self {
        Self::Codex
    }
}

/// All app state in one struct
#[derive(Default)]
pub struct AppState {
    pub current_view: AppView,
    pub tasks: Vec<ReviewTask>,

    pub is_generating: bool,
    pub generation_error: Option<String>,
    pub selected_agent: SelectedAgent,

    pub diff_text: String,

    pub pr_id: String,
    pub pr_title: String,
    pub pr_repo: String,
    pub pr_author: String,
    pub pr_branch: String,

    pub selected_task_id: Option<String>,

    pub agent_messages: Vec<String>,
    pub agent_thoughts: Vec<String>,
    pub agent_logs: Vec<String>,

    pub current_note: Option<String>,
    pub review_error: Option<String>,
}

/// Payload we care about from ACP
pub struct GenResultPayload {
    pub tasks: Vec<ReviewTask>,
    pub messages: Vec<String>,
    pub thoughts: Vec<String>,
    pub logs: Vec<String>,
}

/// Messages coming back from the async generation task
pub enum GenMsg {
    Progress(ProgressEvent),
    Done(Result<GenResultPayload, String>),
}

/// Root egui application
pub struct LaReviewApp {
    pub state: AppState,

    pub task_repo: Arc<TaskRepository>,
    #[allow(dead_code)]
    pub note_repo: Arc<NoteRepository>,
    #[allow(dead_code)]
    pub pr_repo: Arc<PullRequestRepository>,

    pub _db: Database,

    pub gen_tx: mpsc::Sender<GenMsg>,
    pub gen_rx: mpsc::Receiver<GenMsg>,
}

impl LaReviewApp {
    pub fn new_egui(_cc: &eframe::CreationContext<'_>) -> Self {
        // Open database as before
        let db = Database::open().expect("db open");

        let conn = db.connection();
        let task_repo = Arc::new(TaskRepository::new(conn.clone()));
        let note_repo = Arc::new(NoteRepository::new(conn.clone()));
        let pr_repo = Arc::new(PullRequestRepository::new(conn.clone()));

        // Same defaults as original app
        let mut state = AppState::default();
        state.current_view = AppView::Generate;
        state.selected_agent = SelectedAgent::Codex;
        state.diff_text = String::new();
        state.pr_id = "local-pr".to_string();
        state.pr_title = "Local Review".to_string();
        state.pr_repo = "local/repo".to_string();
        state.pr_author = "me".to_string();
        state.pr_branch = "main".to_string();

        let (gen_tx, gen_rx) = mpsc::channel(32);

        Self {
            state,
            task_repo,
            note_repo,
            pr_repo,
            _db: db,
            gen_tx,
            gen_rx,
        }
    }

    /// Switch to review screen
    pub fn switch_to_review(&mut self) {
        self.state.current_view = AppView::Review;
        self.sync_review_from_db();
    }

    /// Switch to generate screen
    pub fn switch_to_generate(&mut self) {
        self.state.current_view = AppView::Generate;
    }

    /// Build a PullRequest struct from current state
    pub fn current_pull_request(&self) -> PullRequest {
        PullRequest {
            id: self.state.pr_id.clone(),
            title: self.state.pr_title.clone(),
            repo: self.state.pr_repo.clone(),
            author: self.state.pr_author.clone(),
            branch: self.state.pr_branch.clone(),
            description: None,
            created_at: String::new(),
        }
    }

    /// Load tasks and notes when switching or refreshing review
    fn sync_review_from_db(&mut self) {
        match self.task_repo.find_by_pr(&self.state.pr_id) {
            Ok(tasks) => {
                self.state.tasks = tasks;
            }
            Err(err) => {
                self.state.review_error = Some(format!("{err}"));
                return;
            }
        }

        match self.note_repo.find_by_tasks(
            &self
                .state
                .tasks
                .iter()
                .map(|t| t.id.clone())
                .collect::<Vec<_>>(),
        ) {
            Ok(notes) => {
                // map notes to current_note if selected
                if let Some(task_id) = &self.state.selected_task_id {
                    self.state.current_note = notes
                        .iter()
                        .find(|n| &n.task_id == task_id)
                        .map(|n| n.body.clone());
                }

                let next_task = self
                    .state
                    .selected_task_id
                    .clone()
                    .filter(|id| self.state.tasks.iter().any(|t| &t.id == id))
                    .or_else(|| self.state.tasks.first().map(|t| t.id.clone()));

                self.state.selected_task_id = next_task;
                self.state.review_error = None;
            }
            Err(err) => {
                self.state.review_error = Some(format!("Failed to load notes: {err}"));
            }
        }
    }

    /// Save the current note for the selected task using real NoteRepository
    pub fn save_current_note(&mut self) {
        let Some(task_id) = &self.state.selected_task_id else {
            return;
        };
        let body = self.state.current_note.clone().unwrap_or_default();

        let timestamp = chrono::Utc::now().to_rfc3339();
        let note = Note {
            task_id: task_id.clone(),
            body: body.clone(),
            updated_at: timestamp,
        };

        let result = self.note_repo.save(&note);

        match result {
            Ok(()) => {
                // Keep state.current_note as-is, clear errors
                self.state.review_error = None;
            }
            Err(err) => {
                self.state.review_error = Some(format!("Failed to save note: {}", err));
            }
        }
    }
}

/// Implement the egui application
impl eframe::App for LaReviewApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // poll async generation messages
        while let Ok(msg) = self.gen_rx.try_recv() {
            match msg {
                GenMsg::Progress(evt) => match evt {
                    ProgressEvent::Message { content, is_new } => {
                        if is_new || self.state.agent_messages.is_empty() {
                            self.state.agent_messages.push(content);
                        } else if let Some(latest) = self.state.agent_messages.last_mut() {
                            *latest = content;
                        }
                    }
                    ProgressEvent::Thought { content, is_new } => {
                        if is_new || self.state.agent_thoughts.is_empty() {
                            self.state.agent_thoughts.push(content);
                        } else if let Some(latest) = self.state.agent_thoughts.last_mut() {
                            *latest = content;
                        }
                    }
                    ProgressEvent::Log(log) => {
                        self.state.agent_logs.push(log);
                    }
                },
                GenMsg::Done(result) => {
                    self.state.is_generating = false;
                    match result {
                        Ok(payload) => {
                            self.state.tasks = payload.tasks;
                            self.state.agent_messages = payload.messages;
                            self.state.agent_thoughts = payload.thoughts;
                            self.state.agent_logs = payload.logs;

                            if self.state.tasks.is_empty() {
                                self.state.generation_error =
                                    Some("No tasks generated".to_string());
                            } else {
                                self.state.current_view = AppView::Review;
                            }
                        }
                        Err(err) => {
                            self.state.generation_error = Some(err);
                        }
                    }
                }
            }
        }

        // top bar (header + nav)
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.heading("LaReview");
            ui.horizontal(|ui| {
                if ui.button("Generate").clicked() {
                    self.switch_to_generate();
                }
                if ui.button("Review").clicked() {
                    self.switch_to_review();
                }
            });
        });

        // main content
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| match self.state.current_view {
                    AppView::Generate => {
                        self.ui_generate(ui);
                    }
                    AppView::Review => {
                        self.ui_review(ui);
                    }
                });
        });
    }
}
