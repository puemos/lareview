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
    SelectRepo(Option<String>),
    ToggleAgentPanel,
    TogglePlanPanel,
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
    UpdateNote {
        note_id: String,
        title: Option<String>,
        severity: Option<crate::domain::NoteSeverity>,
    },
    SaveReply {
        task_id: TaskId,
        parent_id: String,
        root_id: String,
        body: String,
    },
    ResolveThread {
        task_id: TaskId,
        root_id: String,
    },
    SetCurrentNoteText(String),
    StartLineNote(LineNoteContext),
    OpenThread {
        file_path: String,
        line_number: u32,
    },
    OpenAllNotes,
    CloseThread,
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
    LinkRepository,
    UnlinkRepository(String),
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
    ReposLoaded(Result<Vec<crate::domain::LinkedRepo>, String>),
    RepoSaved(Result<crate::domain::LinkedRepo, String>),
    RepoDeleted(Result<String, String>),
    NewRepoPicked(crate::domain::LinkedRepo),
}

#[derive(Debug)]
pub struct ReviewDataPayload {
    pub reviews: Vec<crate::domain::Review>,
    pub runs: Vec<crate::domain::ReviewRun>,
    pub tasks: Vec<crate::domain::ReviewTask>,
}
