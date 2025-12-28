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
    LoadReviewThreads {
        review_id: ReviewId,
    },
    UpdateTaskStatus {
        task_id: TaskId,
        status: ReviewStatus,
    },
    DeleteReview {
        review_id: ReviewId,
    },
    CreateThreadComment {
        review_id: ReviewId,
        task_id: TaskId,
        thread_id: Option<String>,
        file_path: Option<String>,
        line_number: Option<u32>,
        title: Option<String>,
        body: String,
    },
    UpdateThreadStatus {
        thread_id: String,
        status: crate::domain::ReviewStatus,
    },
    UpdateThreadImpact {
        thread_id: String,
        impact: crate::domain::ThreadImpact,
    },
    UpdateThreadTitle {
        thread_id: String,
        title: String,
    },
    RunD2 {
        command: D2Command,
    },
    GenerateExportPreview {
        review_id: String,
        run_id: String,
        include_thread_ids: Option<Vec<String>>,
        options: Box<crate::application::review::export::ExportOptions>,
    },
    ExportReview {
        review_id: ReviewId,
        run_id: crate::domain::ReviewRunId,
        path: std::path::PathBuf,
        options: Box<crate::application::review::export::ExportOptions>,
    },
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
