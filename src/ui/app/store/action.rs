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
    SelectAgent(SelectedAgent),
    ClearTimeline,
}

#[derive(Debug)]
pub enum ReviewAction {
    RefreshFromDb {
        reason: ReviewDataRefreshReason,
    },
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
    CleanDoneTasks,
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
}

#[derive(Debug)]
pub enum SettingsAction {
    SetAllowD2Install(bool),
    RequestD2Install,
    RequestD2Uninstall,
}

#[derive(Debug)]
pub enum AsyncAction {
    GenerationMessage(GenMsg),
    ReviewDataLoaded {
        reason: ReviewDataRefreshReason,
        result: Result<ReviewDataPayload, String>,
    },
    TaskNoteLoaded {
        task_id: String,
        note: Option<String>,
    },
    TaskStatusSaved(Result<(), String>),
    NoteSaved(Result<(), String>),
    DoneTasksCleaned(Result<(), String>),
    D2InstallOutput(String),
    D2InstallComplete,
}

#[derive(Debug)]
pub struct ReviewDataPayload {
    pub prs: Vec<crate::domain::PullRequest>,
    pub tasks: Vec<crate::domain::ReviewTask>,
}
