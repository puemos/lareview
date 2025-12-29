use crate::domain::{ReviewStatus, TaskId};
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
    UpdateDiffText(String),
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
        status: ReviewStatus,
    },
    DeleteReview(String),
    CreateFeedbackComment {
        task_id: TaskId,
        feedback_id: Option<String>,
        file_path: Option<String>,
        line_number: Option<u32>,
        title: Option<String>,
        body: String,
    },
    UpdateFeedbackStatus {
        feedback_id: String,
        status: crate::domain::ReviewStatus,
    },
    UpdateFeedbackImpact {
        feedback_id: String,
        impact: crate::domain::FeedbackImpact,
    },
    UpdateFeedbackTitle {
        feedback_id: String,
        title: String,
    },
    OpenFeedback {
        task_id: TaskId,
        feedback_id: Option<String>,
        file_path: Option<String>,
        line_number: Option<u32>,
    },
    NavigateToFeedback(crate::domain::Feedback),
    CloseFeedback,
    /// User is typing in the feedback title field
    SetFeedbackTitleDraft {
        text: String,
    },
    /// User is typing in the reply composer
    SetFeedbackReplyDraft {
        text: String,
    },
    /// Clear the reply draft after sending
    ClearFeedbackReplyDraft,
    OpenFullDiff(FullDiffView),
    CloseFullDiff,
    RequestExportPreview,
    CloseExportPreview,
    ResetExportCopySuccess,
    ResetExportSaveSuccess,
    ExportReviewToFile {
        path: std::path::PathBuf,
    },
    OpenInEditor {
        file_path: String,
        line_number: usize,
    },
    ToggleFeedbackSelection(String),
    UpdateExportOptions(crate::ui::app::state::ExportOptions),
    ToggleExportOptionsMenu,
    SelectAllExportFeedbacks,
    ClearExportFeedbacks,
}

#[derive(Debug)]
pub enum SettingsAction {
    SetAllowD2Install(bool),
    RequestD2Install,
    RequestD2Uninstall,
    CheckGitHubStatus,
    LinkRepository,
    UnlinkRepository(String),
    DismissRequirements,
    SetPreferredEditor(String),
    OpenEditorPicker,
    ClearPreferredEditor,
    CloseEditorPicker,
    // Agent settings
    UpdateAgentPath(String, String), // agent_id, path
    AddCustomAgent(crate::infra::app_config::CustomAgentConfig),
    RemoveCustomAgent(String),              // agent_id
    UpdateAgentEnv(String, String, String), // agent_id, key, value
    RemoveAgentEnv(String, String),         // agent_id, key
    SaveAgentSettings,
    LoadAgentSettings,
    OpenAgentSettings(String), // agent_id
    CloseAgentSettings,
    OpenAddCustomAgent,
    CloseAddCustomAgent,
}

#[derive(Debug)]
pub enum AsyncAction {
    GenerationMessage(Box<GenMsg>),
    GhStatusLoaded(Result<crate::ui::app::GhStatusPayload, String>),
    ReviewDataLoaded {
        reason: ReviewDataRefreshReason,
        result: Result<ReviewDataPayload, String>,
    },
    ReviewFeedbacksLoaded(Result<ReviewFeedbacksPayload, String>),
    FeedbackCommentSaved(Result<(), String>),
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
pub struct ReviewFeedbacksPayload {
    pub review_id: String,
    pub feedbacks: Vec<crate::domain::Feedback>,
    pub comments: std::collections::HashMap<String, Vec<crate::domain::Comment>>,
}
