use std::collections::{HashMap, HashSet};

use agent_client_protocol::SessionUpdate;

use crate::domain::{Review, ReviewRunId, ReviewTask};
use crate::infra::acp::ProgressEvent;

use super::timeline::{TimelineContent, TimelineItem};

/// Which screen is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppView {
    #[default]
    Generate,
    Review,
    Repos,
    Settings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportOptions {
    pub include_summary: bool,
    pub include_stats: bool,
    pub include_tasks: bool,
    pub include_feedbacks: bool,
    pub include_metadata: bool,
    pub selected_feedback_ids: std::collections::HashSet<String>,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            include_summary: true,
            include_stats: true,
            include_tasks: true,
            include_feedbacks: true,
            include_metadata: true,
            selected_feedback_ids: std::collections::HashSet::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExportOverlayData {
    pub is_exporting: bool,
    pub preview: Option<String>,
    pub options: ExportOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendToPrOverlayData {
    pub selection: HashSet<String>,
    pub include_summary: bool,
    pub pending: bool,
    pub error: Option<String>,
}

impl Default for SendToPrOverlayData {
    fn default() -> Self {
        Self {
            selection: HashSet::new(),
            include_summary: true,
            pending: false,
            error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayState {
    FullDiff(FullDiffView),
    Export(ExportOverlayData),
    /// String is feedback_id
    PushFeedback(String),
    SendToPr(SendToPrOverlayData),
    Requirements,
    EditorPicker,
}

/// Which agent is selected.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SelectedAgent {
    pub id: String,
}

impl SelectedAgent {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

impl std::str::FromStr for SelectedAgent {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self { id: s.to_string() })
    }
}

impl std::fmt::Display for SelectedAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackContext {
    pub feedback_id: Option<String>,
    pub task_id: String,
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
    pub side: Option<crate::domain::FeedbackSide>,
}

#[derive(Debug, Clone)]
pub struct EditorOpenRequest {
    pub file_path: std::path::PathBuf,
    pub line_number: usize,
}

/// Domain-related persistent state.
#[derive(Default)]
pub struct DomainState {
    pub all_tasks: Vec<ReviewTask>,
    pub reviews: Vec<Review>,
    pub runs: Vec<crate::domain::ReviewRun>,
    pub feedbacks: Vec<crate::domain::Feedback>,
    pub feedback_comments: HashMap<String, Vec<crate::domain::Comment>>,
    pub feedback_links: HashMap<String, crate::domain::FeedbackLink>,
    pub linked_repos: Vec<crate::domain::LinkedRepo>,
}

/// State related to the active agent session.
#[derive(Default)]
pub struct SessionState {
    /// Items to display in the agent activity timeline
    pub agent_timeline: Vec<TimelineItem>,
    /// Index for lookups by stream key in the timeline
    pub agent_timeline_index: HashMap<String, usize>,
    /// Monotonic sequence for timeline items
    pub next_agent_timeline_seq: u64,
    /// Flag indicating if an agent is currently running
    pub is_generating: bool,
    /// Last error encountered during generation
    pub generation_error: Option<String>,
    /// Currently selected agent ID
    pub selected_agent: SelectedAgent,
    /// Most recently received generation plan
    pub latest_plan: Option<crate::domain::Plan>,
    /// Text of the diff currently being processed
    pub diff_text: String,
    /// Preview of the generation input (e.g., GH PR meta)
    pub generate_preview: Option<GeneratePreview>,
    /// Flag for async preview fetching
    pub is_preview_fetching: bool,
    /// Last input ref used for fetching a preview
    pub last_preview_input_ref: Option<String>,
    /// Status of GitHub integration
    pub gh_status: Option<crate::ui::app::GhStatusPayload>,
    /// Error from GitHub status check
    pub gh_status_error: Option<String>,
    /// Flag for async GitHub status checking
    pub is_gh_status_checking: bool,
    /// ID of the review currently being generated
    pub generating_review_id: Option<String>,
}

impl SessionState {
    pub fn reset_agent_timeline(&mut self) {
        self.agent_timeline.clear();
        self.agent_timeline_index.clear();
        self.next_agent_timeline_seq = 0;
        self.latest_plan = None;
        self.generating_review_id = None;
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
                let update = &**boxed_update;
                if let SessionUpdate::Plan(plan) = update {
                    self.latest_plan = Some(crate::domain::Plan::from(plan.clone()));
                }

                let key = super::timeline::stream_key_for_update(update);

                if let Some(key) = key {
                    if let Some(&idx) = self.agent_timeline_index.get(&key) {
                        super::timeline::merge_update_in_place(
                            &mut self.agent_timeline[idx],
                            update,
                        );
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

                if let Some(last) = self.agent_timeline.last_mut()
                    && super::timeline::can_merge_contiguous(last, update)
                {
                    super::timeline::merge_update_in_place(last, update);
                    return;
                }

                self.agent_timeline.push(TimelineItem {
                    seq,
                    stream_key: None,
                    content: TimelineContent::Update(Box::new(update.clone())),
                });
            }

            ProgressEvent::Finalized => {
                self.is_generating = false;
                self.generating_review_id = None;
                self.agent_timeline.push(TimelineItem {
                    seq,
                    stream_key: None,
                    content: TimelineContent::LocalLog("Review finalized.".into()),
                });
            }
            ProgressEvent::TaskStarted(id) => {
                self.agent_timeline.push(TimelineItem {
                    seq,
                    stream_key: None,
                    content: TimelineContent::LocalLog(format!("Generating task: {id}")),
                });
            }
            ProgressEvent::TaskAdded(id) => {
                self.agent_timeline.push(TimelineItem {
                    seq,
                    stream_key: None,
                    content: TimelineContent::LocalLog(format!("Task {id} persisted to database.")),
                });
            }
            ProgressEvent::CommentAdded => {
                self.agent_timeline.push(TimelineItem {
                    seq,
                    stream_key: None,
                    content: TimelineContent::LocalLog("Comment persisted to database.".into()),
                });
            }
            ProgressEvent::MetadataUpdated => {
                self.agent_timeline.push(TimelineItem {
                    seq,
                    stream_key: None,
                    content: TimelineContent::LocalLog("Review metadata updated.".into()),
                });
            }
        }
    }
}

use std::sync::Arc;

/// Transient UI state.
#[derive(Default)]
pub struct UiState {
    /// Screen currently being displayed
    pub current_view: AppView,
    /// ID of the selected review
    pub selected_review_id: Option<String>,
    /// ID of the selected generation run
    pub selected_run_id: Option<ReviewRunId>,
    /// ID of the selected review task
    pub selected_task_id: Option<String>,
    /// ID of the selected repository
    pub selected_repo_id: Option<String>,
    /// Error specific to the review view
    pub review_error: Option<String>,
    /// Application-level fatal error
    pub fatal_error: Option<String>,

    /// State of any active modal overlay
    pub active_overlay: Option<OverlayState>,

    /// Cached assets for export (e.g., diagram SVGs)
    pub export_assets: HashMap<String, Vec<u8>>,
    /// Context for creating new feedback or viewing existing one
    pub active_feedback: Option<FeedbackContext>,
    /// Captured output from D2 tool installation
    pub d2_install_output: String,
    /// Flag for async D2 installation
    pub is_d2_installing: bool,
    /// User permission to install D2
    pub allow_d2_install: bool,
    /// Flag to ensure requirements modal is shown once
    pub has_seen_requirements: bool,
    /// User-preferred editor command/ID
    pub preferred_editor_id: Option<String>,
    /// Request to open a file in an external editor
    pub pending_editor_open: Option<EditorOpenRequest>,
    /// Error from editor detection or launching
    pub editor_picker_error: Option<String>,
    /// Flag for clipboard copy success animation
    pub export_copy_success: bool,
    /// Frame counter for copy success hint
    pub export_copy_shown_frames: u8,
    /// Flag for file save success animation
    pub export_save_success: bool,
    /// Frame counter for save success hint
    pub export_save_shown_frames: u8,

    // Feedback → PR sync
    /// Pending feedback ID being pushed to GH
    pub push_feedback_pending: Option<String>,
    /// Last error from GH feedback push
    pub push_feedback_error: Option<String>,
    /// Mapping of review IDs to GH summary comment links
    pub review_summary_links: HashMap<String, String>,

    // Agent settings
    /// User-defined binary paths for agents
    pub agent_path_overrides: std::collections::HashMap<String, String>,
    /// User-defined custom agent configurations
    pub custom_agents: Vec<crate::infra::app_config::CustomAgentConfig>,
    /// Environment variables per agent
    pub agent_envs: std::collections::HashMap<String, std::collections::HashMap<String, String>>,

    // --- Export UI ---
    /// Flag for export options menu visibility
    pub show_export_options_menu: bool,
    /// Content pending to be copied to system clipboard
    pub pending_clipboard_copy: Option<String>,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct AgentSettingsSnapshot {
    pub agent_id: String,
    pub path_override: Option<String>,
    pub envs: Option<std::collections::HashMap<String, String>>,
}

/// All app state in one struct.
#[derive(Default)]
pub struct AppState {
    pub domain: DomainState,
    pub session: SessionState,
    pub ui: UiState,
}

#[derive(Debug, Clone)]
pub struct GeneratePreview {
    pub diff_text: Arc<str>,
    pub github: Option<GitHubPreview>,
}

#[derive(Debug, Clone)]
pub struct GitHubPreview {
    pub pr: crate::infra::vcs::github::GitHubPrRef,
    pub meta: crate::infra::vcs::github::GitHubPrMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullDiffView {
    pub title: String,
    pub text: Arc<str>,
}

impl AppState {
    pub fn reset_agent_timeline(&mut self) {
        self.session.reset_agent_timeline();
    }

    pub fn ingest_progress(&mut self, evt: ProgressEvent) {
        self.session.ingest_progress(evt);
    }

    pub fn tasks(&self) -> Vec<ReviewTask> {
        let Some(selected_run_id) = self.ui.selected_run_id.as_ref() else {
            return self.domain.all_tasks.clone();
        };
        self.domain
            .all_tasks
            .iter()
            .filter(|task| &task.run_id == selected_run_id)
            .cloned()
            .collect()
    }

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
