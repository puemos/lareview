use crate::domain::{TaskId, TaskStatus};
use crate::ui::app::state::AppView;
use crate::ui::app::{FullDiffView, GenMsg, LineNoteContext, SelectedAgent};

use super::command::ReviewDataRefreshReason;

#[derive(Debug)]
pub enum Action {
    Navigation(NavigationAction),
    Generate(GenerateAction),
    Review(ReviewAction),
    Settings(SettingsAction),
    Async(AsyncAction),
}

#[derive(Debug)]
pub enum NavigationAction {
    SwitchTo(AppView),
}

#[derive(Debug)]
pub enum GenerateAction {
    Reset,
    RunRequested,
    FetchPrContext(String),
    SelectAgent(SelectedAgent),
    ClearTimeline,
}

#[derive(Debug)]
pub enum ReviewAction {
    RefreshFromDb {
        reason: ReviewDataRefreshReason,
    },
    SelectReview {
        review_id: String,
    },
    SelectRun {
        run_id: String,
    },
    #[allow(dead_code)]
    RefreshGitHubReview,
    SelectTask {
        task_id: String,
    },
    SelectTaskById {
        task_id: String,
    },
    ClearSelection,
    UpdateTaskStatus {
        task_id: TaskId,
        status: TaskStatus,
    },
    DeleteReview,
    SaveCurrentNote,
    SaveLineNote {
        task_id: TaskId,
        file_path: String,
        line_number: u32,
        body: String,
    },
    SetCurrentNoteText(String),
    StartLineNote(LineNoteContext),
    OpenFullDiff(FullDiffView),
    CloseFullDiff,
    RequestExportPreview,
    CloseExportPreview,
    ExportReviewToFile {
        path: std::path::PathBuf,
    },
}

#[derive(Debug)]
pub enum SettingsAction {
    SetAllowD2Install(bool),
    RequestD2Install,
    RequestD2Uninstall,
    CheckGitHubStatus,
}

#[derive(Debug)]
pub enum AsyncAction {
    GenerationMessage(Box<GenMsg>),
    GhStatusLoaded(Result<crate::ui::app::GhStatusPayload, String>),
    ReviewDataLoaded {
        reason: ReviewDataRefreshReason,
        result: Result<ReviewDataPayload, String>,
    },
    TaskNoteLoaded {
        task_id: String,
        note: Option<String>,
        line_notes: Vec<crate::domain::Note>,
    },
    TaskStatusSaved(Result<(), String>),
    NoteSaved(Result<(), String>),
    ReviewDeleted(Result<(), String>),
    D2InstallOutput(String),
    D2InstallComplete,
    ExportPreviewGenerated(Result<crate::application::review::export::ExportResult, String>),
    ExportFinished(Result<(), String>),
}

#[derive(Debug)]
pub struct ReviewDataPayload {
    pub reviews: Vec<crate::domain::Review>,
    pub runs: Vec<crate::domain::ReviewRun>,
    pub tasks: Vec<crate::domain::ReviewTask>,
}
