//! Main application state and UI logic for the LaReview application
//! This module contains the primary egui application state and UI implementation

use std::sync::Arc;

use crate::acp::ProgressEvent;
use crate::data::db::Database;
use crate::data::repository::{NoteRepository, PullRequestRepository, TaskRepository};
use crate::domain::{Note, PullRequest, ReviewTask};

use catppuccin_egui::MOCHA;
use eframe::egui;
use eframe::egui::{FontData, FontDefinitions, FontFamily};
use tokio::sync::mpsc;

/// Which screen is active
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppView {
    #[default]
    Generate,
    Review,
}

/// Which agent is selected (matches original code)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectedAgent {
    #[default]
    Codex,
    Gemini,
    Qwen,
}

/// All app state in one struct
#[derive(Default)]
pub struct AppState {
    pub current_view: AppView,
    // Renamed tasks to all_tasks to hold all tasks regardless of PR
    /// All review tasks fetched from the database, to be filtered by selected PR
    pub all_tasks: Vec<ReviewTask>,

    /// Flag indicating if task generation is currently in progress
    pub is_generating: bool,
    /// Error message from failed task generation, if any
    pub generation_error: Option<String>,
    /// Currently selected agent for task generation
    pub selected_agent: SelectedAgent,

    /// Current diff text in the generate view
    pub diff_text: String,

    /// All pull requests loaded from the database
    pub prs: Vec<PullRequest>,
    /// ID of the currently selected pull request
    pub selected_pr_id: Option<String>,

    /// Fields representing the current PR context
    /// These fields are dynamically updated based on the selected PR
    pub pr_id: String,
    pub pr_title: String,
    pub pr_repo: String,
    pub pr_author: String,
    pub pr_branch: String,

    /// ID of the currently selected review task
    pub selected_task_id: Option<String>,

    /// Messages from the agent during task generation
    pub agent_messages: Vec<String>,
    /// Thoughts from the agent during task generation
    pub agent_thoughts: Vec<String>,
    /// Log messages from the agent during task generation
    pub agent_logs: Vec<String>,

    /// Current note content for the selected task
    pub current_note: Option<String>,
    /// Context for the currently active line note (when user clicks on a line to comment)
    pub current_line_note: Option<LineNoteContext>,
    /// Error message from review operations, if any
    pub review_error: Option<String>,

    /// Current full diff view state, if any
    pub full_diff: Option<FullDiffView>,

    /// Cache for unified diff string to prevent expensive re-parsing on every frame
    pub cached_unified_diff: Option<(Vec<crate::domain::Patch>, String)>,
}

#[derive(Debug, Clone)]
pub struct FullDiffView {
    pub title: String,
    pub text: String,
}

/// Context for tracking an active line note being created
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LineNoteContext {
    pub task_id: String,
    pub file_idx: usize,
    pub line_idx: usize,
    pub line_number: usize,
    pub file_path: String,  // New field to store the file path
    pub note_text: String,  // The actual text being typed
}

impl AppState {
    /// Get tasks filtered by the currently selected pull request ID
    pub fn tasks(&self) -> Vec<ReviewTask> {
        if let Some(selected_pr_id) = &self.selected_pr_id {
            self.all_tasks
                .iter()
                .filter(|task| task.pr_id == *selected_pr_id)
                .cloned()
                .collect()
        } else {
            // When no specific PR is selected, we might want to display all tasks, or none.
            // For now, let's display all tasks if no PR is selected.
            // In the future, this could be an empty vec or a specific "All PRs" view.
            self.all_tasks.clone()
        }
    }

    /// Group tasks by sub-flows for the selected pull request
    pub fn tasks_by_sub_flow(&self) -> std::collections::HashMap<Option<String>, Vec<ReviewTask>> {
        let tasks = self.tasks();
        let mut grouped: std::collections::HashMap<Option<String>, Vec<ReviewTask>> =
            std::collections::HashMap::new();

        for task in tasks {
            grouped.entry(task.sub_flow.clone()).or_default().push(task);
        }

        grouped
    }
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

/// Root egui application for LaReview
pub struct LaReviewApp {
    /// Application state containing UI state, tasks, PRs, etc.
    pub state: AppState,

    /// Repository for task operations
    pub task_repo: Arc<TaskRepository>,
    /// Repository for note operations
    pub note_repo: Arc<NoteRepository>,
    /// Repository for pull request operations
    pub pr_repo: Arc<PullRequestRepository>,

    /// Database connection wrapper (kept to maintain connection during app lifetime)
    pub _db: Database,

    /// Sender for async generation messages
    pub gen_tx: mpsc::Sender<GenMsg>,
    /// Receiver for async generation messages
    pub gen_rx: mpsc::Receiver<GenMsg>,
}

impl LaReviewApp {
    pub fn new_egui(cc: &eframe::CreationContext<'_>) -> Self {
        // Configure fonts
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "SpaceMono".to_owned(),
            FontData::from_static(include_bytes!("../../assets/fonts/SpaceMono-Regular.ttf"))
                .into(),
        );
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "SpaceMono".to_owned());
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "SpaceMono".to_owned());

        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

        cc.egui_ctx.set_fonts(fonts);

        // Open database as before
        let db = Database::open().expect("db open");

        let conn = db.connection();
        let task_repo = Arc::new(TaskRepository::new(conn.clone()));
        let note_repo = Arc::new(NoteRepository::new(conn.clone()));
        let pr_repo = Arc::new(PullRequestRepository::new(conn.clone()));

        // Initial state
        let mut state = AppState {
            current_view: AppView::Generate,
            selected_agent: SelectedAgent::Codex,
            diff_text: String::new(),
            pr_id: "local-pr".to_string(), // Default local PR
            pr_title: "Local Review".to_string(),
            pr_repo: "local/repo".to_string(),
            pr_author: "me".to_string(),
            pr_branch: "main".to_string(),
            ..Default::default()
        };

        // Load all PRs and set selected PR
        if let Ok(prs) = pr_repo.list_all() {
            state.prs = prs;
            if let Some(first_pr) = state.prs.first() {
                state.selected_pr_id = Some(first_pr.id.clone());
                state.pr_id = first_pr.id.clone();
                state.pr_title = first_pr.title.clone();
                state.pr_repo = first_pr.repo.clone();
                state.pr_author = first_pr.author.clone();
                state.pr_branch = first_pr.branch.clone();
            }
        } else {
            state.review_error = Some("Failed to load pull requests".to_string());
        }

        let (gen_tx, gen_rx) = mpsc::channel(32);

        let mut app = Self {
            state,
            task_repo,
            note_repo,
            pr_repo,
            _db: db,
            gen_tx,
            gen_rx,
        };

        // Load the app logo texture once at startup
        if let Ok(image_bytes) = std::fs::read("assets/icons/icon-256.png")
            && let Ok(image) = image::load_from_memory(&image_bytes)
        {
            let size = [image.width() as usize, image.height() as usize];
            let rgba = image.to_rgba8();
            let pixels = rgba.as_raw();

            let _logo_handle = cc.egui_ctx.load_texture(
                "app_logo",
                egui::ColorImage::from_rgba_unmultiplied(size, pixels),
                egui::TextureOptions::LINEAR,
            );
        }

        app.sync_review_from_db(); // Load tasks for the initial state

        app
    }

    /// Switch to review screen and load review data from database
    pub fn switch_to_review(&mut self) {
        self.state.current_view = AppView::Review;
        self.sync_review_from_db();
    }

    /// Switch to generate screen
    pub fn switch_to_generate(&mut self) {
        self.state.current_view = AppView::Generate;
    }

    /// Build a PullRequest struct from current state or selected PR
    pub fn current_pull_request(&self) -> PullRequest {
        // If a PR is selected, use its details, otherwise use the local-pr defaults
        if let Some(selected_pr_id) = &self.state.selected_pr_id
            && let Some(pr) = self.state.prs.iter().find(|p| &p.id == selected_pr_id)
        {
            return pr.clone();
        }
        PullRequest {
            id: self.state.pr_id.clone(), // Fallback to current state's pr_id (local-pr)
            title: self.state.pr_title.clone(),
            repo: self.state.pr_repo.clone(),
            author: self.state.pr_author.clone(),
            branch: self.state.pr_branch.clone(),
            description: None,
            created_at: String::new(),
        }
    }

    /// Load tasks and notes when switching or refreshing review
    pub fn sync_review_from_db(&mut self) {
        // Load all PRs
        match self.pr_repo.list_all() {
            Ok(prs) => {
                self.state.prs = prs;
                // If selected_pr_id is invalid or not set, try to set it to the first PR
                if self.state.selected_pr_id.is_none()
                    || !self
                        .state
                        .prs
                        .iter()
                        .any(|p| Some(&p.id) == self.state.selected_pr_id.as_ref())
                {
                    if let Some(first_pr) = self.state.prs.first() {
                        self.state.selected_pr_id = Some(first_pr.id.clone());
                    } else {
                        self.state.selected_pr_id = None; // No PRs available
                    }
                }
            }
            Err(err) => {
                self.state.review_error = Some(format!("Failed to load pull requests: {}", err));
                return;
            }
        }

        // Load all tasks
        match self.task_repo.find_all() {
            // Use find_all to get all tasks
            Ok(all_tasks) => {
                self.state.all_tasks = all_tasks;

                // Update current PR details based on selected_pr_id
                if let Some(selected_pr_id) = &self.state.selected_pr_id {
                    if let Some(pr) = self.state.prs.iter().find(|p| &p.id == selected_pr_id) {
                        self.state.pr_id = pr.id.clone();
                        self.state.pr_title = pr.title.clone();
                        self.state.pr_repo = pr.repo.clone();
                        self.state.pr_author = pr.author.clone();
                        self.state.pr_branch = pr.branch.clone();
                    }
                } else {
                    // Reset to default local-pr if no PR is selected
                    self.state.pr_id = "local-pr".to_string();
                    self.state.pr_title = "Local Review".to_string();
                    self.state.pr_repo = "local/repo".to_string();
                    self.state.pr_author = "me".to_string();
                    self.state.pr_branch = "main".to_string();
                }
            }
            Err(err) => {
                self.state.review_error = Some(format!("Failed to load tasks: {}", err));
                return;
            }
        }

        // After loading/filtering, re-evaluate selected task and note
        let current_tasks = self.state.tasks(); // This calls the getter which filters
        let next_task = self
            .state
            .selected_task_id
            .clone()
            .filter(|id| current_tasks.iter().any(|t| &t.id == id))
            .or_else(|| current_tasks.first().map(|t| t.id.clone()));

        self.state.selected_task_id = next_task;

        if let Some(task_id) = &self.state.selected_task_id {
            if let Ok(Some(note)) = self.note_repo.find_by_task(task_id) {
                self.state.current_note = Some(note.body);
            } else {
                self.state.current_note = Some(String::new());
            }
        } else {
            self.state.current_note = None;
        }

        self.state.review_error = None;
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
            file_path: None,
            line_number: None,
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
        // Set Catppuccin theme
        catppuccin_egui::set_theme(ctx, MOCHA);

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
                            self.state.all_tasks = payload.tasks; // Changed to all_tasks
                            self.state.agent_messages = payload.messages;
                            self.state.agent_thoughts = payload.thoughts;
                            self.state.agent_logs = payload.logs;

                            if self.state.all_tasks.is_empty() {
                                // Changed to all_tasks
                                self.state.generation_error =
                                    Some("No tasks generated".to_string());
                            } else {
                                self.switch_to_review();
                            }
                        }
                        Err(err) => {
                            self.state.generation_error = Some(err);
                        }
                    }
                }
            }
        }

        // Top panel with app title and navigation
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            // Add a simple, subtle diagonal line background decoration
            let rect = ui.available_rect_before_wrap();
            ui.painter().with_clip_rect(rect).rect_filled(
                rect,
                egui::CornerRadius::ZERO,
                MOCHA.base, // Use base background color
            );

            // Create a simple diagonal line pattern in the background to avoid rendering issues on hover
            let line_spacing = 25.0;
            let line_width = 0.4;
            let color = MOCHA.surface0.linear_multiply(0.25);

            // Draw diagonal lines from top-left to bottom-right
            let mut pos = rect.min.x - rect.max.y;
            while pos < rect.max.x {
                ui.painter().line_segment(
                    [
                        egui::Pos2::new(pos, rect.min.y),
                        egui::Pos2::new(pos + (rect.max.y - rect.min.y), rect.max.y),
                    ],
                    egui::Stroke::new(line_width, color),
                );
                pos += line_spacing;
            }

            // Draw diagonal lines from bottom-left to top-right for X pattern
            let mut pos = rect.min.y;
            while pos < rect.max.y {
                ui.painter().line_segment(
                    [
                        egui::Pos2::new(rect.min.x, pos),
                        egui::Pos2::new(rect.min.x + (rect.max.y - pos), rect.max.y),
                    ],
                    egui::Stroke::new(line_width, color),
                );
                pos += line_spacing;
            }

            ui.add_space(12.0); // Left padding
            ui.horizontal(|ui| {
                // App Title with LaReview logo
                ui.horizontal(|ui| {
                    // Display the app logo
                    match ui.ctx().try_load_texture(
                        "app_logo",
                        egui::TextureOptions::LINEAR,
                        Default::default(),
                    ) {
                        Ok(egui::load::TexturePoll::Ready { texture }) => {
                            ui.image(texture);
                        }
                        Ok(egui::load::TexturePoll::Pending { .. }) => {
                            // Texture is still loading, show placeholder
                            ui.add(egui::Label::new(
                                egui::RichText::new(egui_phosphor::regular::CIRCLE_HALF)
                                    .size(22.0)
                                    .color(MOCHA.mauve),
                            ));
                        }
                        Err(_) => {
                            // Texture failed to load, show fallback
                            ui.add(egui::Label::new(
                                egui::RichText::new(egui_phosphor::regular::CIRCLE_HALF)
                                    .size(22.0)
                                    .color(MOCHA.mauve),
                            ));
                        }
                    }

                    ui.add_space(8.0);
                    ui.heading(
                        egui::RichText::new("LaReview")
                            .strong()
                            .color(MOCHA.text)
                            .size(18.0),
                    );
                });

                ui.add_space(20.0); // Space from title to navigation

                // Navigation Buttons - left aligned after logo
                ui.horizontal(|ui| {
                    // Generate Button
                    let generate_response = ui.add(
                        egui::Button::new(egui::RichText::new("GENERATE").color(
                            if self.state.current_view == AppView::Generate {
                                MOCHA.mauve // Highlight active view
                            } else {
                                MOCHA.subtext1 // Softer color for inactive
                            },
                        ))
                        .frame(false)
                        .corner_radius(egui::CornerRadius::same(4)),
                    );
                    if generate_response.clicked() {
                        self.switch_to_generate();
                    }

                    ui.add_space(12.0); // Space between buttons

                    // Review Button
                    let review_response = ui.add(
                        egui::Button::new(egui::RichText::new("REVIEW").color(
                            if self.state.current_view == AppView::Review {
                                MOCHA.mauve // Highlight active view
                            } else {
                                MOCHA.subtext1 // Softer color for inactive
                            },
                        ))
                        .frame(false)
                        .corner_radius(egui::CornerRadius::same(4)),
                    );
                    if review_response.clicked() {
                        self.switch_to_review();
                    }
                });
            });
            ui.add_space(8.0); // Bottom padding for better vertical spacing
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

        // Full diff overlay window
        if let Some(full) = self.state.full_diff.clone() {
            // Fill the root viewport
            let viewport_rect = ctx.input(|i| i.viewport().inner_rect).unwrap_or_else(|| {
                let rect = ctx.available_rect();
                egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), rect.size())
            });

            let mut open = true;

            egui::Window::new(full.title.clone())
                .open(&mut open)
                .fixed_rect(viewport_rect)
                .collapsible(false)
                .resizable(false)
                .title_bar(true)
                .show(ctx, |ui| {
                    ui.spacing_mut().window_margin = egui::Margin::symmetric(12, 8);
                    ui.horizontal(|ui| {
                        if ui
                            .button(format!("{} Close", egui_phosphor::regular::ARROW_SQUARE_IN))
                            .clicked()
                        {
                            // Mark for close
                            self.state.full_diff = None;
                        }
                    });

                    ui.separator();

                    // The big diff area itself
                    crate::ui::components::diff::render_diff_editor_full_view(
                        ui, &full.text, "diff",
                    );
                });

            if !open {
                self.state.full_diff = None;
            }
        }
    }
}
