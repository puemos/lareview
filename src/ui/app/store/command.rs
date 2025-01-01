use crate::domain::{ReviewId, ReviewStatus, TaskId};
use crate::infra::acp::RunContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewDataRefreshReason {
    Manual,
    Navigation,
    AfterGeneration,
    AfterStatusChange,
    AfterReviewDelete,
    Incremental,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum D2Command {
    Install,
    Uninstall,
}

#[derive(Debug, Clone)]
pub enum Command {
    ResolveGenerateInput {
        input_text: String,
        selected_agent_id: String,
        review_id: Option<String>,
    },
    FetchPrContextPreview {
        input_ref: String,
    },
    AbortGeneration,
    CheckGitHubStatus,
    RefreshGitHubReview {
        review_id: String,
        selected_agent_id: String,
    },
    StartGeneration {
        run_context: Box<RunContext>,
        selected_agent_id: String,
    },
    RefreshReviewData {
        reason: ReviewDataRefreshReason,
    },
    LoadReviewFeedbacks {
        review_id: ReviewId,
    },
    LoadFeedbackLinks {
        review_id: ReviewId,
    },
    UpdateTaskStatus {
        task_id: TaskId,
        status: ReviewStatus,
    },
    DeleteReview {
        review_id: ReviewId,
    },
    CreateFeedbackComment {
        review_id: ReviewId,
        task_id: TaskId,
        feedback_id: Option<String>,
        file_path: Option<String>,
        line_number: Option<u32>,
        side: Option<crate::domain::FeedbackSide>,
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
    SendFeedbackToPr {
        feedback_id: String,
    },
    RunD2 {
        command: D2Command,
    },
    GenerateExportPreview {
        review_id: String,
        run_id: String,
        include_feedback_ids: Option<Vec<String>>,
        options: Box<crate::application::review::export::ExportOptions>,
    },
    ExportReview {
        review_id: ReviewId,
        run_id: crate::domain::ReviewRunId,
        path: std::path::PathBuf,
        options: Box<crate::application::review::export::ExportOptions>,
    },
    DeleteFeedback(String),
    DeleteComment(String),
    SaveRepo {
        repo: crate::domain::LinkedRepo,
    },
    DeleteRepo {
        repo_id: String,
    },
    PickFolderForLink,
    SaveAppConfigFull {
        has_seen_requirements: bool,
        custom_agents: Vec<crate::infra::app_config::CustomAgentConfig>,
        agent_path_overrides: std::collections::HashMap<String, String>,
        agent_envs: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
        preferred_editor_id: Option<String>,
    },
    OpenInEditor {
        editor_id: String,
        file_path: std::path::PathBuf,
        line_number: usize,
    },
}
