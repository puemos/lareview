//! Main application state and UI logic for the LaReview application
//! This module contains the primary egui application state and UI implementation

use std::collections::HashMap;
use std::sync::Arc;

use crate::acp::ProgressEvent;
use crate::data::db::Database;
use crate::data::repository::{NoteRepository, PullRequestRepository, TaskRepository};
use crate::domain::{Note, PullRequest, ReviewTask, TaskId, TaskStatus};

use catppuccin_egui::MOCHA;
use eframe::egui;
use eframe::egui::{FontData, FontDefinitions, FontFamily};
use agent_client_protocol::{ContentBlock, ContentChunk, Meta, Plan, SessionUpdate};
use tokio::sync::mpsc;

/// Which screen is active
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppView {
    #[default]
    Generate,
    Review,
    Settings,
}

/// Which agent is selected (matches original code)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SelectedAgent {
    pub id: String,
}

impl SelectedAgent {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    pub fn from_str(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}

/// All app state in one struct
#[derive(Default)]
pub struct AppState {
    pub current_view: AppView,
    // Renamed tasks to all_tasks to hold all tasks regardless of PR
    /// All review tasks fetched from the database, to be filtered by selected PR
    pub all_tasks: Vec<ReviewTask>,

    /// Canonical unified timeline of agent session updates.
    pub agent_timeline: Vec<TimelineItem>,
    /// In-memory index: stream key -> timeline item index.
    pub agent_timeline_index: HashMap<String, usize>,
    /// Monotonic counter for deterministic ordering at ingestion.
    pub next_agent_timeline_seq: u64,

    /// Flag indicating if task generation is currently in progress
    pub is_generating: bool,
    /// Error message from failed task generation, if any
    pub generation_error: Option<String>,
    /// Currently selected agent for task generation
    pub selected_agent: SelectedAgent,

    /// Current diff text in the generate view
    pub diff_text: String,

    /// Latest agent-provided plan (if any)
    pub latest_plan: Option<Plan>,

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

    /// Current note content for the selected task
    pub current_note: Option<String>,
    /// Context for the currently active line note (when user clicks on a line to comment)
    pub current_line_note: Option<LineNoteContext>,
    /// Error message from review operations, if any
    pub review_error: Option<String>,

    /// Current full diff view state, if any
    pub full_diff: Option<FullDiffView>,

    /// Cache for unified diff string to prevent expensive re-parsing on every frame
    pub cached_unified_diff: Option<(Vec<String>, String)>,

    /// Output of the D2 installation command
    pub d2_install_output: String,
    /// Flag indicating if D2 installation is in progress
    pub is_d2_installing: bool,
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
    pub file_path: String, // New field to store the file path
    pub note_text: String, // The actual text being typed
}

impl AppState {
    pub fn reset_agent_timeline(&mut self) {
        self.agent_timeline.clear();
        self.agent_timeline_index.clear();
        self.next_agent_timeline_seq = 0;
        self.latest_plan = None;
    }

    pub fn ingest_progress(&mut self, evt: ProgressEvent) {
        let seq = self.next_agent_timeline_seq;
        self.next_agent_timeline_seq = self.next_agent_timeline_seq.saturating_add(1);

        match evt {
            ProgressEvent::LocalLog(line) => {
                self.agent_timeline.push(TimelineItem {
                    seq,
                    stream_key: None,
                    content: TimelineContent::LocalLog(line),
                });
            }
            ProgressEvent::Update(ref boxed_update) => {
                let update = &**boxed_update; // Dereference the box to get SessionUpdate
                if let SessionUpdate::Plan(plan) = update {
                    self.latest_plan = Some(plan.clone());
                }

                let key = stream_key_for_update(update);

                if let Some(key) = key {
                    if let Some(&idx) = self.agent_timeline_index.get(&key) {
                        merge_update_in_place(&mut self.agent_timeline[idx], update);
                        return;
                    }
                    let idx = self.agent_timeline.len();
                    self.agent_timeline_index.insert(key.clone(), idx);
                    self.agent_timeline.push(TimelineItem {
                        seq,
                        stream_key: Some(key),
                        content: TimelineContent::Update(Box::new(update.clone())),
                    });
                    return;
                }

                // No stable key: attempt to merge with last contiguous chunk of same kind.
                if let Some(last) = self.agent_timeline.last_mut()
                    && can_merge_contiguous(last, update) {
                    merge_update_in_place(last, update);
                    return;
                }

                self.agent_timeline.push(TimelineItem {
                    seq,
                    stream_key: None,
                    content: TimelineContent::Update(Box::new(update.clone())),
                });
            }
        }
    }

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

#[derive(Debug, Clone)]
pub struct TimelineItem {
    #[allow(dead_code)]
    pub seq: u64,
    pub stream_key: Option<String>,
    pub content: TimelineContent,
}

#[derive(Debug, Clone)]
pub enum TimelineContent {
    Update(Box<SessionUpdate>),
    LocalLog(String),
}

fn stream_key_for_update(update: &SessionUpdate) -> Option<String> {
    match update {
        SessionUpdate::UserMessageChunk(chunk) => {
            extract_chunk_id(chunk.meta.as_ref()).map(|id| format!("user_msg:{id}"))
        }
        SessionUpdate::AgentMessageChunk(chunk) => {
            extract_chunk_id(chunk.meta.as_ref()).map(|id| format!("agent_msg:{id}"))
        }
        SessionUpdate::AgentThoughtChunk(chunk) => {
            extract_chunk_id(chunk.meta.as_ref()).map(|id| format!("agent_thought:{id}"))
        }
        SessionUpdate::ToolCall(call) => Some(format!("tool:{}", call.tool_call_id)),
        SessionUpdate::ToolCallUpdate(update) => Some(format!("tool:{}", update.tool_call_id)),
        SessionUpdate::Plan(_) => Some("plan".to_string()),
        _ => None,
    }
}

fn extract_chunk_id(meta: Option<&Meta>) -> Option<String> {
    meta.and_then(|meta| {
        ["message_id", "messageId", "id"]
            .iter()
            .find_map(|key| meta.get(*key).and_then(|val| val.as_str()))
            .map(|s| s.to_string())
    })
}

fn can_merge_contiguous(existing: &TimelineItem, incoming: &SessionUpdate) -> bool {
    if existing.stream_key.is_some() {
        return false;
    }
    match (&existing.content, incoming) {
        (TimelineContent::Update(boxed_update), SessionUpdate::AgentMessageChunk(_)) if matches!(**boxed_update, SessionUpdate::AgentMessageChunk(_)) => true,
        (TimelineContent::Update(boxed_update), SessionUpdate::AgentThoughtChunk(_)) if matches!(**boxed_update, SessionUpdate::AgentThoughtChunk(_)) => true,
        (TimelineContent::Update(boxed_update), SessionUpdate::UserMessageChunk(_)) if matches!(**boxed_update, SessionUpdate::UserMessageChunk(_)) => true,
        _ => false,
    }
}

fn merge_update_in_place(existing: &mut TimelineItem, incoming: &SessionUpdate) {
    match (&mut existing.content, incoming) {
        (TimelineContent::Update(boxed_update), SessionUpdate::AgentMessageChunk(next)) => {
            if let SessionUpdate::AgentMessageChunk(prev) = &mut **boxed_update {
                merge_content_chunk(prev, next);
            } else {
                existing.content = TimelineContent::Update(Box::new(incoming.clone()));
            }
        }
        (TimelineContent::Update(boxed_update), SessionUpdate::AgentThoughtChunk(next)) => {
            if let SessionUpdate::AgentThoughtChunk(prev) = &mut **boxed_update {
                merge_content_chunk(prev, next);
            } else {
                existing.content = TimelineContent::Update(Box::new(incoming.clone()));
            }
        }
        (TimelineContent::Update(boxed_update), SessionUpdate::UserMessageChunk(next)) => {
            if let SessionUpdate::UserMessageChunk(prev) = &mut **boxed_update {
                merge_content_chunk(prev, next);
            } else {
                existing.content = TimelineContent::Update(Box::new(incoming.clone()));
            }
        }
        (TimelineContent::Update(boxed_update), SessionUpdate::ToolCallUpdate(update)) => {
            if let SessionUpdate::ToolCall(call) = &mut **boxed_update {
                call.update(update.fields.clone());
            } else {
                existing.content = TimelineContent::Update(Box::new(incoming.clone()));
            }
        }
        (TimelineContent::Update(boxed_update), SessionUpdate::ToolCall(call)) => {
            if let SessionUpdate::ToolCallUpdate(_existing_update) = &mut **boxed_update {
                // Convert ToolCallUpdate to ToolCall
                **boxed_update = SessionUpdate::ToolCall(call.clone());
            } else if let SessionUpdate::ToolCall(existing_call) = &mut **boxed_update {
                *existing_call = call.clone();
            } else {
                existing.content = TimelineContent::Update(Box::new(SessionUpdate::ToolCall(call.clone())));
            }
        }
        (TimelineContent::Update(boxed_update), SessionUpdate::Plan(plan)) => {
            if let SessionUpdate::Plan(existing_plan) = &mut **boxed_update {
                *existing_plan = plan.clone();
            } else {
                existing.content = TimelineContent::Update(Box::new(SessionUpdate::Plan(plan.clone())));
            }
        }
        (_, _) => {
            existing.content = TimelineContent::Update(Box::new(incoming.clone()));
        }
    }
}

fn merge_content_chunk(existing: &mut ContentChunk, incoming: &ContentChunk) {
    match (&mut existing.content, &incoming.content) {
        (ContentBlock::Text(prev_text), ContentBlock::Text(next_text)) => {
            prev_text.text.push_str(&next_text.text);
        }
        _ => {
            existing.content = incoming.content.clone();
        }
    }
    if existing.meta.is_none() {
        existing.meta = incoming.meta.clone();
    }
}

/// Payload we care about from ACP
#[allow(dead_code)]
pub struct GenResultPayload {
    pub messages: Vec<String>,
    pub thoughts: Vec<String>,
    pub logs: Vec<String>,
}

/// Messages coming back from the async generation task
pub enum GenMsg {
    Progress(Box<ProgressEvent>),
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

    /// Sender for D2 installation messages
    pub d2_install_tx: mpsc::Sender<String>,
    /// Receiver for D2 installation messages
    pub d2_install_rx: mpsc::Receiver<String>,
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

        // Install image loaders for SVG and other formats
        egui_extras::install_image_loaders(&cc.egui_ctx);

        // Open database as before
        let db = Database::open().expect("db open");

        let conn = db.connection();
        let task_repo = Arc::new(TaskRepository::new(conn.clone()));
        let note_repo = Arc::new(NoteRepository::new(conn.clone()));
        let pr_repo = Arc::new(PullRequestRepository::new(conn.clone()));

        // Initial state
        let mut state = AppState {
            current_view: AppView::Generate,
            selected_agent: SelectedAgent::new("codex"),
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
        let (d2_install_tx, d2_install_rx) = mpsc::channel(32);

        let mut app = Self {
            state,
            task_repo,
            note_repo,
            pr_repo,
            _db: db,
            gen_tx,
            gen_rx,
            d2_install_tx,
            d2_install_rx,
        };

        // Load the app logo texture once at startup
        if let Ok(image_bytes) = std::fs::read("assets/icons/icon-512.png")
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

    /// Switch to settings screen
    pub fn switch_to_settings(&mut self) {
        self.state.current_view = AppView::Settings;
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
        self.state.selected_task_id = self
            .state
            .selected_task_id
            .clone()
            .filter(|id| current_tasks.iter().any(|t| &t.id == id));

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

    pub fn set_task_status(&mut self, task_id: &TaskId, new_status: TaskStatus) {
        if let Err(err) = self.task_repo.update_status(task_id, new_status) {
            self.state.review_error = Some(format!("Failed to update task status: {}", err));
            return;
        }

        self.state.selected_task_id = Some(task_id.clone());
        self.state.review_error = None;
        self.sync_review_from_db();
    }

    pub fn clean_done_tasks(&mut self) {
        let Some(pr_id) = self.state.selected_pr_id.clone() else {
            return;
        };

        let done_ids = match self.task_repo.find_done_ids_by_pr(&pr_id) {
            Ok(ids) => ids,
            Err(err) => {
                self.state.review_error =
                    Some(format!("Failed to list done tasks: {}", err));
                return;
            }
        };

        if done_ids.is_empty() {
            return;
        }

        if let Err(err) = self.note_repo.delete_by_task_ids(&done_ids) {
            self.state.review_error =
                Some(format!("Failed to delete notes for done tasks: {}", err));
            return;
        }

        if let Err(err) = self.task_repo.delete_by_ids(&done_ids) {
            self.state.review_error =
                Some(format!("Failed to delete done tasks: {}", err));
            return;
        }

        self.state.review_error = None;
        self.state.selected_task_id = None;
        self.state.current_note = None;
        self.state.current_line_note = None;
        self.sync_review_from_db();
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

        // poll D2 installation messages
        while let Ok(msg) = self.d2_install_rx.try_recv() {
            if msg == "___INSTALL_COMPLETE___" {
                self.state.is_d2_installing = false;
            } else {
                self.state.d2_install_output.push_str(&msg);
                self.state.d2_install_output.push('\n');
            }
        }

        // poll async generation messages
        let mut agent_content_updated = false;
        while let Ok(msg) = self.gen_rx.try_recv() {
            match msg {
                GenMsg::Progress(evt) => {
                    self.state.ingest_progress(*evt);
                    agent_content_updated = true;
                }
                GenMsg::Done(result) => {
                    self.state.is_generating = false;
                    match result {
                        Ok(_payload) => {
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
                    agent_content_updated = true;
                }
            }
        }

        // Request repaint if agent content was updated OR if we're still generating
        if agent_content_updated || self.state.is_generating {
            // Use request_repaint_after for smoother updates without constant repainting
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
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
            let line_spacing = 20.0;
            let line_width = 0.5;
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

                    ui.add_space(2.0);
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

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Settings Button
                        let settings_response = ui.add(
                            egui::Button::new(egui::RichText::new("SETTINGS").color(
                                if self.state.current_view == AppView::Settings {
                                    MOCHA.mauve // Highlight active view
                                } else {
                                    MOCHA.subtext1 // Softer color for inactive
                                },
                            ))
                            .frame(false)
                            .corner_radius(egui::CornerRadius::same(4)),
                        );
                        if settings_response.clicked() {
                            self.switch_to_settings();
                        }
                    });
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
                    AppView::Settings => {
                        self.ui_settings(ui);
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

            let outer_padding = egui::vec2(12.0, 8.0);

            egui::Window::new(full.title.clone())
                .open(&mut open)
                // space between the window and the viewport edges
                .fixed_rect(viewport_rect.shrink2(outer_padding))
                // padding inside the window frame (affects separator, +102, etc.)
                .frame(
                    egui::Frame::window(&ctx.style()).inner_margin(egui::Margin::symmetric(12, 8)),
                )
                .collapsible(false)
                .resizable(false)
                .title_bar(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .button(format!("{} Close", egui_phosphor::regular::ARROW_SQUARE_IN))
                            .clicked()
                        {
                            self.state.full_diff = None;
                        }
                    });

                    ui.separator();

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
