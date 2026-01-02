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

#[derive(Default)]
pub struct DomainState {
    pub all_tasks: Vec<ReviewTask>,
    pub reviews: Vec<Review>,
    pub runs: Vec<crate::domain::ReviewRun>,
    pub feedbacks: Vec<crate::domain::Feedback>,
    pub feedback_comments: HashMap<String, Vec<crate::domain::Comment>>,
    pub feedback_links: HashMap<String, crate::domain::FeedbackLink>,
    pub linked_repos: Vec<crate::domain::LinkedRepo>,
    pub pending_review: Option<PendingReviewState>,
}

#[derive(Debug, Clone)]
pub struct PendingReviewState {
    pub diff: String,
    pub repo_root: Option<std::path::PathBuf>,
    pub agent: Option<String>,
    pub auto_generate: bool,
    pub source: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Default)]
pub struct SessionState {
    pub agent_timeline: Vec<TimelineItem>,
    pub agent_timeline_index: HashMap<String, usize>,
    pub next_agent_timeline_seq: u64,
    pub is_generating: bool,
    pub generation_error: Option<String>,
    pub selected_agent: SelectedAgent,
    pub latest_plan: Option<crate::domain::Plan>,
    pub diff_text: String,
    pub generate_preview: Option<GeneratePreview>,
    pub is_preview_fetching: bool,
    pub last_preview_input_ref: Option<String>,
    pub gh_status: Option<crate::ui::app::GhStatusPayload>,
    pub gh_status_error: Option<String>,
    pub is_gh_status_checking: bool,
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

#[derive(Default)]
pub struct UiState {
    pub current_view: AppView,
    pub selected_review_id: Option<String>,
    pub selected_run_id: Option<ReviewRunId>,
    pub selected_task_id: Option<String>,
    pub selected_repo_id: Option<String>,
    pub review_error: Option<String>,
    pub fatal_error: Option<String>,
    pub active_overlay: Option<OverlayState>,
    pub export_assets: HashMap<String, Vec<u8>>,
    pub active_feedback: Option<FeedbackContext>,
    pub d2_install_output: String,
    pub is_d2_installing: bool,
    pub allow_d2_install: bool,
    pub has_seen_requirements: bool,
    pub preferred_editor_id: Option<String>,
    pub pending_editor_open: Option<EditorOpenRequest>,
    pub editor_picker_error: Option<String>,
    pub export_copy_success: bool,
    pub export_copy_shown_frames: u8,
    pub export_save_success: bool,
    pub export_save_shown_frames: u8,
    pub push_feedback_pending: Option<String>,
    pub push_feedback_error: Option<String>,
    pub review_summary_links: HashMap<String, String>,
    pub agent_path_overrides: std::collections::HashMap<String, String>,
    pub custom_agents: Vec<crate::infra::app_config::CustomAgentConfig>,
    pub agent_envs: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    pub show_export_options_menu: bool,
    pub pending_clipboard_copy: Option<String>,
    pub cli_install_output: String,
    pub is_cli_installing: bool,
    pub cli_install_success: bool,
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
