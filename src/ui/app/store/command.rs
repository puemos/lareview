use crate::domain::{PullRequest, TaskId, TaskStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewDataRefreshReason {
    Manual,
    Navigation,
    AfterGeneration,
    AfterStatusChange,
    AfterCleanup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum D2Command {
    Install,
    Uninstall,
}

#[derive(Debug, Clone)]
pub enum Command {
    StartGeneration {
        pull_request: Box<PullRequest>,
        diff_text: String,
        selected_agent_id: String,
    },
    RefreshReviewData {
        reason: ReviewDataRefreshReason,
    },
    LoadTaskNote {
        task_id: TaskId,
    },
    UpdateTaskStatus {
        task_id: TaskId,
        status: TaskStatus,
    },
    CleanDoneTasks {
        pr_id: Option<String>,
    },
    SaveNote {
        task_id: TaskId,
        body: String,
        file_path: Option<String>,
        line_number: Option<u32>,
    },
    RunD2 {
        command: D2Command,
    },
}
