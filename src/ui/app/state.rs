use egui_commonmark::CommonMarkCache;
use std::collections::HashMap;

use agent_client_protocol::{Plan, SessionUpdate};

use crate::domain::{Review, ReviewRunId, ReviewTask};
use crate::infra::acp::ProgressEvent;

use super::timeline::{TimelineContent, TimelineItem};

/// Which screen is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppView {
    #[default]
    Generate,
    Review,
    Settings,
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

/// All app state in one struct.
#[derive(Default)]
pub struct AppState {
    pub markdown_cache: CommonMarkCache,
    pub current_view: AppView,
    /// All review tasks fetched from the database, to be filtered by selected PR.
    pub all_tasks: Vec<ReviewTask>,

    /// Canonical unified timeline of agent session updates.
    pub agent_timeline: Vec<TimelineItem>,
    /// In-memory index: stream key -> timeline item index.
    pub agent_timeline_index: HashMap<String, usize>,
    /// Monotonic counter for deterministic ordering at ingestion.
    pub next_agent_timeline_seq: u64,

    pub is_generating: bool,
    pub generation_error: Option<String>,
    pub selected_agent: SelectedAgent,

    pub diff_text: String,
    pub generate_preview: Option<GeneratePreview>,
    pub is_preview_fetching: bool,
    pub last_preview_input_ref: Option<String>,
    pub gh_status: Option<crate::ui::app::GhStatusPayload>,
    pub gh_status_error: Option<String>,
    pub is_gh_status_checking: bool,

    /// Latest agent-provided plan (if any).
    pub latest_plan: Option<Plan>,

    pub reviews: Vec<Review>,
    pub runs: Vec<crate::domain::ReviewRun>,
    pub selected_review_id: Option<String>,
    pub selected_run_id: Option<ReviewRunId>,

    pub selected_task_id: Option<String>,

    pub current_note: Option<String>,
    pub task_line_notes: Vec<crate::domain::Note>,
    pub current_line_note: Option<LineNoteContext>,
    pub review_error: Option<String>,

    pub full_diff: Option<FullDiffView>,

    pub export_preview: Option<String>,
    pub export_assets: HashMap<String, Vec<u8>>,

    pub cached_unified_diff: Option<(Vec<crate::domain::DiffRef>, String)>,

    pub is_exporting: bool,
    pub d2_install_output: String,
    pub is_d2_installing: bool,
    pub allow_d2_install: bool,
}

#[derive(Debug, Clone)]
pub struct GeneratePreview {
    pub diff_text: String,
    pub github: Option<GitHubPreview>,
}

#[derive(Debug, Clone)]
pub struct GitHubPreview {
    pub pr: crate::infra::github::GitHubPrRef,
    pub meta: crate::infra::github::GitHubPrMetadata,
}

#[derive(Debug, Clone)]
pub struct FullDiffView {
    pub title: String,
    pub text: String,
}

/// Context for tracking an active line note being created.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LineNoteContext {
    pub task_id: String,
    pub file_idx: usize,
    pub line_idx: usize,
    pub line_number: usize,
    pub file_path: String,
    pub note_text: String,
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
                let update = &**boxed_update;
                if let SessionUpdate::Plan(plan) = update {
                    self.latest_plan = Some(plan.clone());
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
                self.agent_timeline.push(TimelineItem {
                    seq,
                    stream_key: None,
                    content: TimelineContent::LocalLog("Review finalized.".into()),
                });
            }
        }
    }

    pub fn tasks(&self) -> Vec<ReviewTask> {
        let Some(selected_run_id) = self.selected_run_id.as_ref() else {
            return self.all_tasks.clone();
        };
        self.all_tasks
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
