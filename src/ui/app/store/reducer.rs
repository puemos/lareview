use super::super::state::{AppState, AppView};
use super::action::{
    Action, AsyncAction, GenerateAction, NavigationAction, ReviewAction, ReviewDataPayload,
    SettingsAction,
};
use super::command::{Command, D2Command, ReviewDataRefreshReason};
use crate::domain::{TaskId, TaskStatus};
use crate::ui::app::GenMsg;

pub fn reduce(state: &mut AppState, action: Action) -> Vec<Command> {
    match action {
        Action::Navigation(action) => reduce_navigation(state, action),
        Action::Generate(action) => reduce_generate(state, action),
        Action::Review(action) => reduce_review(state, action),
        Action::Settings(action) => reduce_settings(state, action),
        Action::Async(action) => reduce_async(state, action),
    }
}

fn reduce_navigation(state: &mut AppState, action: NavigationAction) -> Vec<Command> {
    match action {
        NavigationAction::SwitchTo(view) => {
            state.current_view = view;
            if matches!(view, AppView::Review) {
                return vec![Command::RefreshReviewData {
                    reason: ReviewDataRefreshReason::Navigation,
                }];
            }
            Vec::new()
        }
    }
}

fn reduce_generate(state: &mut AppState, action: GenerateAction) -> Vec<Command> {
    match action {
        GenerateAction::Reset => {
            state.diff_text.clear();
            state.generation_error = None;
            state.is_generating = false;
            state.reset_agent_timeline();
            state.current_view = AppView::Generate;
            Vec::new()
        }
        GenerateAction::RunRequested => {
            if state.diff_text.trim().is_empty() {
                state.generation_error = Some("Please paste a git diff first".into());
                return Vec::new();
            }

            state.generation_error = None;
            state.is_generating = true;
            state.reset_agent_timeline();
            vec![Command::StartGeneration {
                pull_request: Box::new(pull_request_from_state(state)),
                diff_text: state.diff_text.clone(),
                selected_agent_id: state.selected_agent.id.clone(),
            }]
        }
        GenerateAction::SelectAgent(agent) => {
            state.selected_agent = agent;
            Vec::new()
        }
        GenerateAction::ClearTimeline => {
            state.reset_agent_timeline();
            Vec::new()
        }
    }
}

fn reduce_review(state: &mut AppState, action: ReviewAction) -> Vec<Command> {
    match action {
        ReviewAction::RefreshFromDb { reason } => vec![Command::RefreshReviewData { reason }],
        ReviewAction::SelectTask { task_id } => select_task(state, task_id),
        ReviewAction::SelectTaskById { task_id } => {
            if state.tasks().iter().any(|t| t.id == task_id) {
                select_task(state, task_id)
            } else {
                Vec::new()
            }
        }
        ReviewAction::ClearSelection => {
            state.selected_task_id = None;
            state.current_note = None;
            state.current_line_note = None;
            state.cached_unified_diff = None;
            Vec::new()
        }
        ReviewAction::UpdateTaskStatus { task_id, status } => {
            state.review_error = None;
            vec![Command::UpdateTaskStatus { task_id, status }]
        }
        ReviewAction::CleanDoneTasks => {
            state.review_error = None;
            vec![Command::CleanDoneTasks {
                pr_id: state.selected_pr_id.clone(),
            }]
        }
        ReviewAction::SaveCurrentNote => {
            let Some(task_id) = state.selected_task_id.clone() else {
                return Vec::new();
            };
            let body = state.current_note.clone().unwrap_or_default();
            state.review_error = None;
            vec![Command::SaveNote {
                task_id,
                body,
                file_path: None,
                line_number: None,
            }]
        }
        ReviewAction::SaveLineNote {
            task_id,
            file_path,
            line_number,
            body,
        } => {
            state.current_line_note = None;
            state.review_error = None;
            vec![Command::SaveNote {
                task_id,
                body,
                file_path: Some(file_path),
                line_number: Some(line_number),
            }]
        }
        ReviewAction::SetCurrentNoteText(text) => {
            state.current_note = Some(text);
            Vec::new()
        }
        ReviewAction::StartLineNote(ctx) => {
            state.current_line_note = Some(ctx);
            state.current_note = None;
            Vec::new()
        }
        ReviewAction::OpenFullDiff(view) => {
            state.full_diff = Some(view);
            Vec::new()
        }
        ReviewAction::CloseFullDiff => {
            state.full_diff = None;
            Vec::new()
        }
    }
}

fn reduce_settings(state: &mut AppState, action: SettingsAction) -> Vec<Command> {
    match action {
        SettingsAction::SetAllowD2Install(allow) => {
            state.allow_d2_install = allow;
            Vec::new()
        }
        SettingsAction::RequestD2Install => {
            if !state.allow_d2_install || state.is_d2_installing {
                return Vec::new();
            }
            state.is_d2_installing = true;
            state.d2_install_output.clear();
            vec![Command::RunD2 {
                command: D2Command::Install,
            }]
        }
        SettingsAction::RequestD2Uninstall => {
            if !state.allow_d2_install || state.is_d2_installing {
                return Vec::new();
            }
            state.is_d2_installing = true;
            state.d2_install_output.clear();
            vec![Command::RunD2 {
                command: D2Command::Uninstall,
            }]
        }
    }
}

fn reduce_async(state: &mut AppState, action: AsyncAction) -> Vec<Command> {
    match action {
        AsyncAction::GenerationMessage(msg) => reduce_generation_msg(state, msg),
        AsyncAction::ReviewDataLoaded { reason, result } => match result {
            Ok(payload) => {
                let commands = apply_review_data(state, payload);

                if matches!(reason, ReviewDataRefreshReason::AfterGeneration) {
                    if state.tasks().is_empty() {
                        state.current_view = AppView::Generate;
                        state.generation_error = Some("No tasks generated".to_string());
                    } else {
                        state.current_view = AppView::Review;
                        state.generation_error = None;
                    }
                }

                commands
            }
            Err(err) => {
                state.review_error = Some(err);
                Vec::new()
            }
        },
        AsyncAction::TaskNoteLoaded { task_id, note } => {
            if state.selected_task_id.as_ref() == Some(&task_id) {
                state.current_note = Some(note.unwrap_or_default());
                state.review_error = None;
            }
            Vec::new()
        }
        AsyncAction::TaskStatusSaved(result) => {
            if let Err(err) = result {
                state.review_error = Some(err);
                Vec::new()
            } else {
                state.review_error = None;
                vec![Command::RefreshReviewData {
                    reason: ReviewDataRefreshReason::AfterStatusChange,
                }]
            }
        }
        AsyncAction::NoteSaved(result) => {
            if let Err(err) = result {
                state.review_error = Some(err);
            } else {
                state.review_error = None;
            }
            Vec::new()
        }
        AsyncAction::DoneTasksCleaned(result) => {
            if let Err(err) = result {
                state.review_error = Some(err);
                Vec::new()
            } else {
                state.review_error = None;
                state.selected_task_id = None;
                state.current_note = None;
                state.current_line_note = None;
                vec![Command::RefreshReviewData {
                    reason: ReviewDataRefreshReason::AfterCleanup,
                }]
            }
        }
        AsyncAction::D2InstallOutput(line) => {
            state.d2_install_output.push_str(&line);
            state.d2_install_output.push('\n');
            Vec::new()
        }
        AsyncAction::D2InstallComplete => {
            state.is_d2_installing = false;
            Vec::new()
        }
    }
}

fn reduce_generation_msg(state: &mut AppState, msg: GenMsg) -> Vec<Command> {
    match msg {
        GenMsg::Progress(evt) => {
            state.ingest_progress(*evt);
            Vec::new()
        }
        GenMsg::Done(result) => {
            state.is_generating = false;

            match result {
                Ok(_payload) => vec![Command::RefreshReviewData {
                    reason: ReviewDataRefreshReason::AfterGeneration,
                }],
                Err(err) => {
                    state.generation_error = Some(err);
                    Vec::new()
                }
            }
        }
    }
}

fn apply_review_data(state: &mut AppState, payload: ReviewDataPayload) -> Vec<Command> {
    state.prs = payload.prs;
    state.all_tasks = payload.tasks;
    state.review_error = None;
    state.cached_unified_diff = None;

    if let Some(selected) = &state.selected_pr_id
        && !state.prs.iter().any(|p| &p.id == selected)
    {
        state.selected_pr_id = None;
    }

    if state.selected_pr_id.is_none() {
        state.selected_pr_id = state.prs.first().map(|p| p.id.clone());
    }

    if let Some(selected_pr_id) = &state.selected_pr_id {
        if let Some(pr) = state.prs.iter().find(|p| &p.id == selected_pr_id) {
            state.pr_id = pr.id.clone();
            state.pr_title = pr.title.clone();
            state.pr_repo = pr.repo.clone();
            state.pr_author = pr.author.clone();
            state.pr_branch = pr.branch.clone();
        }
    } else {
        state.pr_id = "local-pr".to_string();
        state.pr_title = "Local Review".to_string();
        state.pr_repo = "local/repo".to_string();
        state.pr_author = "me".to_string();
        state.pr_branch = "main".to_string();
    }

    let current_tasks = state.tasks();

    if let Some(selected_task_id) = &state.selected_task_id
        && !current_tasks.iter().any(|t| &t.id == selected_task_id)
    {
        state.selected_task_id = None;
        state.current_note = None;
        state.current_line_note = None;
    }

    let mut commands = Vec::new();

    if let Some(task_id) = &state.selected_task_id {
        commands.push(Command::LoadTaskNote {
            task_id: task_id.clone(),
        });
    } else if let Some(next_open) = current_tasks
        .iter()
        .find(|t| matches!(t.status, TaskStatus::Pending | TaskStatus::InProgress))
    {
        state.selected_task_id = Some(next_open.id.clone());
        state.current_line_note = None;
        state.current_note = Some(String::new());
        commands.push(Command::LoadTaskNote {
            task_id: next_open.id.clone(),
        });
    }

    commands
}

fn select_task(state: &mut AppState, task_id: TaskId) -> Vec<Command> {
    state.selected_task_id = Some(task_id.clone());
    state.current_line_note = None;
    state.cached_unified_diff = None;
    state.current_note = Some(String::new());
    vec![Command::LoadTaskNote { task_id }]
}

fn pull_request_from_state(state: &AppState) -> crate::domain::PullRequest {
    if let Some(selected_pr_id) = &state.selected_pr_id
        && let Some(pr) = state.prs.iter().find(|p| &p.id == selected_pr_id)
    {
        return pr.clone();
    }
    crate::domain::PullRequest {
        id: state.pr_id.clone(),
        title: state.pr_title.clone(),
        repo: state.pr_repo.clone(),
        author: state.pr_author.clone(),
        branch: state.pr_branch.clone(),
        description: None,
        created_at: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{PullRequest, ReviewTask, TaskStats, TaskStatus};
    use crate::ui::app::{GenMsg, GenResultPayload, SelectedAgent};

    fn pending_task(id: &str, pr_id: &str) -> ReviewTask {
        ReviewTask {
            id: id.to_string(),
            pr_id: pr_id.to_string(),
            title: "Task".into(),
            description: "Desc".into(),
            files: vec![],
            stats: TaskStats::default(),
            diffs: vec![],
            insight: None,
            diagram: None,
            ai_generated: false,
            status: TaskStatus::Pending,
            sub_flow: None,
        }
    }

    #[test]
    fn generate_run_requested_emits_command() {
        let mut state = AppState {
            diff_text: "diff --git a b".into(),
            selected_agent: SelectedAgent::new("agent-1"),
            ..Default::default()
        };

        let commands = reduce(&mut state, Action::Generate(GenerateAction::RunRequested));

        assert!(state.is_generating);
        assert!(
            matches!(
                commands.as_slice(),
                [Command::StartGeneration { selected_agent_id, .. }]
                if selected_agent_id == "agent-1"
            ),
            "expected StartGeneration command"
        );
    }

    #[test]
    fn generation_done_triggers_review_refresh() {
        let mut state = AppState {
            is_generating: true,
            ..Default::default()
        };

        let commands = reduce(
            &mut state,
            Action::Async(AsyncAction::GenerationMessage(GenMsg::Done(Ok(
                GenResultPayload {
                    messages: vec![],
                    thoughts: vec![],
                    logs: vec![],
                },
            )))),
        );

        assert!(!state.is_generating);
        assert!(
            matches!(
                commands.as_slice(),
                [Command::RefreshReviewData {
                    reason: ReviewDataRefreshReason::AfterGeneration
                }]
            ),
            "expected review refresh command after generation"
        );
    }

    #[test]
    fn review_data_load_selects_first_pending() {
        let pr = PullRequest {
            id: "pr1".into(),
            title: "PR".into(),
            description: None,
            repo: "r".into(),
            author: "a".into(),
            branch: "b".into(),
            created_at: "now".into(),
        };
        let task = pending_task("t1", "pr1");
        let mut state = AppState::default();

        let commands = reduce(
            &mut state,
            Action::Async(AsyncAction::ReviewDataLoaded {
                reason: ReviewDataRefreshReason::Manual,
                result: Ok(ReviewDataPayload {
                    prs: vec![pr.clone()],
                    tasks: vec![task.clone()],
                }),
            }),
        );

        assert_eq!(state.pr_id, pr.id);
        assert_eq!(state.pr_title, pr.title);
        assert_eq!(state.selected_task_id.as_deref(), Some("t1"));
        assert!(
            matches!(
                commands.as_slice(),
                [Command::LoadTaskNote { task_id }]
                if task_id == "t1"
            ),
            "expected note load for first pending task"
        );
    }

    #[test]
    fn update_task_status_emits_command() {
        let mut state = AppState::default();
        let commands = reduce(
            &mut state,
            Action::Review(ReviewAction::UpdateTaskStatus {
                task_id: "t1".into(),
                status: TaskStatus::Done,
            }),
        );

        assert!(
            matches!(
                commands.as_slice(),
                [Command::UpdateTaskStatus { task_id, status: TaskStatus::Done }]
                if task_id == "t1"
            ),
            "expected status update command"
        );
        assert!(state.review_error.is_none());
    }

    #[test]
    fn settings_install_only_when_allowed() {
        let mut state = AppState {
            allow_d2_install: false,
            is_d2_installing: false,
            ..Default::default()
        };

        let none = reduce(
            &mut state,
            Action::Settings(SettingsAction::RequestD2Install),
        );
        assert!(none.is_empty());
        assert!(!state.is_d2_installing);

        state.allow_d2_install = true;
        let commands = reduce(
            &mut state,
            Action::Settings(SettingsAction::RequestD2Install),
        );
        assert!(state.is_d2_installing);
        assert!(
            matches!(
                commands.as_slice(),
                [Command::RunD2 {
                    command: D2Command::Install
                }]
            ),
            "expected install command when allowed"
        );
    }

    #[test]
    fn navigation_to_review_triggers_refresh() {
        let mut state = AppState::default();
        let commands = reduce(
            &mut state,
            Action::Navigation(NavigationAction::SwitchTo(AppView::Review)),
        );

        assert_eq!(state.current_view, AppView::Review);
        assert!(
            matches!(
                commands.as_slice(),
                [Command::RefreshReviewData {
                    reason: ReviewDataRefreshReason::Navigation
                }]
            ),
            "expected refresh when entering review"
        );
    }

    #[test]
    fn clean_done_tasks_enqueues_command() {
        let mut state = AppState {
            selected_pr_id: Some("pr1".into()),
            ..Default::default()
        };

        let commands = reduce(&mut state, Action::Review(ReviewAction::CleanDoneTasks));
        assert!(state.review_error.is_none());
        assert!(
            matches!(
                commands.as_slice(),
                [Command::CleanDoneTasks { pr_id }]
                if pr_id.as_deref() == Some("pr1")
            ),
            "expected clean-done command"
        );
    }

    #[test]
    fn review_data_loaded_without_prs_defaults_to_local() {
        let mut state = AppState::default();
        let commands = reduce(
            &mut state,
            Action::Async(AsyncAction::ReviewDataLoaded {
                reason: ReviewDataRefreshReason::Manual,
                result: Ok(ReviewDataPayload {
                    prs: vec![],
                    tasks: vec![],
                }),
            }),
        );

        assert_eq!(state.pr_id, "local-pr");
        assert!(state.selected_pr_id.is_none());
        assert!(commands.is_empty(), "no note load without tasks");
    }

    #[test]
    fn review_data_after_generation_with_no_tasks_stays_on_generate() {
        let mut state = AppState {
            current_view: AppView::Generate,
            ..Default::default()
        };

        let commands = reduce(
            &mut state,
            Action::Async(AsyncAction::ReviewDataLoaded {
                reason: ReviewDataRefreshReason::AfterGeneration,
                result: Ok(ReviewDataPayload {
                    prs: vec![],
                    tasks: vec![],
                }),
            }),
        );

        assert!(commands.is_empty());
        assert_eq!(state.current_view, AppView::Generate);
        assert_eq!(
            state.generation_error.as_deref(),
            Some("No tasks generated")
        );
    }

    #[test]
    fn set_current_note_text_updates_state_only() {
        let mut state = AppState::default();
        let commands = reduce(
            &mut state,
            Action::Review(ReviewAction::SetCurrentNoteText("hello".into())),
        );
        assert_eq!(state.current_note.as_deref(), Some("hello"));
        assert!(commands.is_empty());
    }
}
