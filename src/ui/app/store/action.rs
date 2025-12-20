use crate::domain::{TaskId, TaskStatus};
use crate::ui::app::state::AppView;
use crate::ui::app::{FullDiffView, GenMsg, SelectedAgent};

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
    CreateThreadComment {
        task_id: TaskId,
        thread_id: Option<String>,
        file_path: Option<String>,
        line_number: Option<u32>,
        title: Option<String>,
        body: String,
    },
    UpdateThreadStatus {
        thread_id: String,
        status: crate::domain::ThreadStatus,
    },
    UpdateThreadImpact {
        thread_id: String,
        impact: crate::domain::ThreadImpact,
    },
    UpdateThreadTitle {
        thread_id: String,
        title: String,
    },
    OpenThread {
        task_id: TaskId,
        thread_id: Option<String>,
        file_path: Option<String>,
        line_number: Option<u32>,
    },
    CloseThread,
    /// User is typing in the thread title field
    SetThreadTitleDraft {
        text: String,
    },
    /// User is typing in the reply composer
    SetThreadReplyDraft {
        text: String,
    },
    /// Clear the reply draft after sending
    ClearThreadReplyDraft,
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
    ReviewThreadsLoaded(Result<ReviewThreadsPayload, String>),
    ThreadCommentSaved(Result<(), String>),
    TaskStatusSaved(Result<(), String>),
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

#[derive(Debug)]
pub struct ReviewThreadsPayload {
    pub review_id: String,
    pub threads: Vec<crate::domain::Thread>,
    pub comments: std::collections::HashMap<String, Vec<crate::domain::Comment>>,
}
