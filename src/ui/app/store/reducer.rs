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
            if matches!(view, AppView::Settings) {
                // If we haven't checked GitHub status yet, trigger it
                if state.gh_status.is_none()
                    && state.gh_status_error.is_none()
                    && !state.is_gh_status_checking
                {
                    state.is_gh_status_checking = true;
                    return vec![Command::CheckGitHubStatus];
                }
            }
            Vec::new()
        }
    }
}

fn reduce_generate(state: &mut AppState, action: GenerateAction) -> Vec<Command> {
    match action {
        GenerateAction::Reset => {
            state.diff_text.clear();
            state.generate_preview = None;
            state.is_preview_fetching = false;
            state.last_preview_input_ref = None;
            state.generation_error = None;
            state.is_generating = false;
            state.reset_agent_timeline();
            state.current_view = AppView::Generate;
            Vec::new()
        }
        GenerateAction::RunRequested => {
            if state.diff_text.trim().is_empty() {
                state.generation_error =
                    Some("Please paste a diff or GitHub PR reference first".into());
                return Vec::new();
            }

            state.generation_error = None;
            state.is_generating = true;
            state.reset_agent_timeline();
            state.generate_preview = None;
            vec![Command::ResolveGenerateInput {
                input_text: state.diff_text.clone(),
                selected_agent_id: state.selected_agent.id.clone(),
                review_id: None,
            }]
        }

        GenerateAction::FetchPrContext(input_ref) => {
            let input_ref = input_ref.trim().to_string();
            if input_ref.is_empty() {
                return Vec::new();
            }
            if state.is_preview_fetching {
                return Vec::new();
            }
            if state.last_preview_input_ref.as_deref() == Some(input_ref.as_str())
                && state.generate_preview.is_some()
            {
                return Vec::new();
            }

            state.is_preview_fetching = true;
            state.last_preview_input_ref = Some(input_ref.clone());
            state.generation_error = None;
            state.generate_preview = None;

            vec![Command::FetchPrContextPreview { input_ref }]
        }
        GenerateAction::SelectAgent(agent) => {
            state.selected_agent = agent;
            Vec::new()
        }
        GenerateAction::ClearTimeline => {
            state.reset_agent_timeline();
            Vec::new()
        }
        GenerateAction::SelectRepo(repo_id) => {
            state.selected_repo_id = repo_id;
            Vec::new()
        }
        GenerateAction::ToggleAgentPanel => {
            state.agent_panel_collapsed = !state.agent_panel_collapsed;
            Vec::new()
        }
        GenerateAction::TogglePlanPanel => {
            state.plan_panel_collapsed = !state.plan_panel_collapsed;
            Vec::new()
        }
    }
}

fn reduce_review(state: &mut AppState, action: ReviewAction) -> Vec<Command> {
    match action {
        ReviewAction::RefreshFromDb { reason } => vec![Command::RefreshReviewData { reason }],
        ReviewAction::RefreshGitHubReview => {
            let Some(review_id) = state.selected_review_id.clone() else {
                return Vec::new();
            };
            let Some(review) = state.reviews.iter().find(|r| r.id == review_id) else {
                return Vec::new();
            };
            if !matches!(review.source, crate::domain::ReviewSource::GitHubPr { .. }) {
                state.review_error = Some("Selected review is not a GitHub PR".into());
                return Vec::new();
            }

            state.review_error = None;
            state.generation_error = None;
            state.is_generating = true;
            state.reset_agent_timeline();
            state.generate_preview = None;
            state.current_view = AppView::Generate;

            vec![Command::RefreshGitHubReview {
                review_id,
                selected_agent_id: state.selected_agent.id.clone(),
            }]
        }
        ReviewAction::SelectReview { review_id } => {
            state.selected_review_id = Some(review_id.clone());
            state.selected_run_id = state
                .reviews
                .iter()
                .find(|r| r.id == review_id)
                .and_then(|r| r.active_run_id.clone());

            state.selected_task_id = None;
            state.current_note = None;
            state.current_line_note = None;
            state.cached_unified_diff = None;

            select_default_task_for_current_run(state)
        }
        ReviewAction::SelectRun { run_id } => {
            state.selected_run_id = Some(run_id);
            state.selected_task_id = None;
            state.current_note = None;
            state.current_line_note = None;
            state.cached_unified_diff = None;
            select_default_task_for_current_run(state)
        }
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
        ReviewAction::DeleteReview => {
            if let Some(review_id) = state.selected_review_id.clone() {
                state.review_error = None;
                vec![Command::DeleteReview { review_id }]
            } else {
                Vec::new()
            }
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
                parent_id: None,
                root_id: None,
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
                parent_id: None,
                root_id: None,
            }]
        }
        ReviewAction::SaveReply {
            task_id,
            parent_id,
            root_id,
            body,
        } => {
            state.review_error = None;
            vec![Command::SaveNote {
                task_id,
                body,
                file_path: None, // Replies are linked by parent_id/root_id
                line_number: None,
                parent_id: Some(parent_id),
                root_id: Some(root_id),
            }]
        }
        ReviewAction::ResolveThread { task_id, root_id } => {
            state.review_error = None;
            vec![Command::ResolveThread { task_id, root_id }]
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
        ReviewAction::OpenThread {
            file_path,
            line_number,
        } => {
            state.active_thread = Some(crate::ui::app::ThreadContext {
                file_path,
                line_number,
            });
            Vec::new()
        }
        ReviewAction::OpenAllNotes => {
            // Deprecated/Removed functionality
            Vec::new()
        }
        ReviewAction::CloseThread => {
            state.active_thread = None;
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
        ReviewAction::RequestExportPreview => {
            if let Some(review_id) = state.selected_review_id.clone()
                && let Some(run_id) = state.selected_run_id.clone()
            {
                state.is_exporting = true;
                state.review_error = None;
                vec![Command::GenerateExportPreview { review_id, run_id }]
            } else {
                state.review_error = Some("No review or run selected for export".into());
                vec![]
            }
        }
        ReviewAction::CloseExportPreview => {
            state.export_preview = None;
            Vec::new()
        }
        ReviewAction::ExportReviewToFile { path } => {
            if let (Some(review_id), Some(run_id)) =
                (&state.selected_review_id, &state.selected_run_id)
            {
                state.is_exporting = true;
                vec![Command::ExportReview {
                    review_id: review_id.clone(),
                    run_id: run_id.clone(),
                    path,
                }]
            } else {
                Vec::new()
            }
        }
        ReviewAction::UpdateNote {
            note_id,
            title,
            severity,
        } => {
            vec![Command::UpdateNote {
                note_id,
                title,
                severity,
            }]
        }
    }
}

fn reduce_settings(state: &mut AppState, action: SettingsAction) -> Vec<Command> {
    match action {
        SettingsAction::SetAllowD2Install(allow) => {
            state.allow_d2_install = allow;
            Vec::new()
        }
        SettingsAction::CheckGitHubStatus => {
            if state.is_gh_status_checking {
                return Vec::new();
            }
            state.is_gh_status_checking = true;
            state.gh_status_error = None;
            vec![Command::CheckGitHubStatus]
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
        SettingsAction::LinkRepository => {
            vec![Command::PickFolderForLink]
        }
        SettingsAction::UnlinkRepository(repo_id) => {
            vec![Command::DeleteRepo { repo_id }]
        }
    }
}

fn reduce_async(state: &mut AppState, action: AsyncAction) -> Vec<Command> {
    match action {
        AsyncAction::GenerationMessage(msg) => reduce_generation_msg(state, *msg),
        AsyncAction::GhStatusLoaded(result) => {
            state.is_gh_status_checking = false;
            match result {
                Ok(status) => {
                    state.gh_status = Some(status);
                    state.gh_status_error = None;
                }
                Err(err) => {
                    state.gh_status = None;
                    state.gh_status_error = Some(err);
                }
            }
            Vec::new()
        }
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
        AsyncAction::TaskNoteLoaded {
            task_id,
            note,
            line_notes,
        } => {
            if state.selected_task_id.as_ref() == Some(&task_id) {
                state.current_note = Some(note.unwrap_or_default());
                state.task_line_notes = line_notes;
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
        AsyncAction::ReviewDeleted(result) => {
            if let Err(err) = result {
                state.review_error = Some(err);
                Vec::new()
            } else {
                state.review_error = None;
                state.selected_review_id = None;
                state.selected_run_id = None;
                state.selected_task_id = None;
                state.current_note = None;
                state.current_line_note = None;
                vec![Command::RefreshReviewData {
                    reason: ReviewDataRefreshReason::AfterReviewDelete,
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
        AsyncAction::ExportPreviewGenerated(result) => {
            state.is_exporting = false;
            match result {
                Ok(res) => {
                    state.export_preview = Some(res.markdown);
                    state.export_assets = res.assets;
                }
                Err(err) => {
                    state.review_error = Some(format!("Failed to generate preview: {}", err));
                }
            }
            Vec::new()
        }
        AsyncAction::ExportFinished(result) => {
            state.is_exporting = false;
            if let Err(e) = result {
                state.review_error = Some(format!("Export failed: {}", e));
            } else {
                state.export_preview = None;
            }
            vec![]
        }
        AsyncAction::ReposLoaded(result) => {
            match result {
                Ok(repos) => state.linked_repos = repos,
                Err(err) => state.review_error = Some(format!("Failed to load repos: {}", err)),
            }
            Vec::new()
        }
        AsyncAction::RepoSaved(result) => {
            match result {
                Ok(repo) => {
                    state.linked_repos.push(repo);
                }
                Err(err) => state.review_error = Some(format!("Failed to save repo: {}", err)),
            }
            Vec::new()
        }
        AsyncAction::RepoDeleted(result) => {
            match result {
                Ok(repo_id) => {
                    state.linked_repos.retain(|r| r.id != repo_id);
                    if state.selected_repo_id.as_ref() == Some(&repo_id) {
                        state.selected_repo_id = None;
                    }
                }
                Err(err) => state.review_error = Some(format!("Failed to delete repo: {}", err)),
            }
            Vec::new()
        }
        AsyncAction::NewRepoPicked(repo) => {
            vec![Command::SaveRepo { repo }]
        }
    }
}

fn reduce_generation_msg(state: &mut AppState, msg: GenMsg) -> Vec<Command> {
    match msg {
        GenMsg::PreviewResolved { input_ref, result } => {
            state.is_preview_fetching = false;
            if state.diff_text.trim() != input_ref.trim() {
                return Vec::new();
            }
            match result {
                Ok(preview) => {
                    state.generate_preview = Some(preview.clone());
                    state.generation_error = None;

                    if let Some(github) = &preview.github {
                        // Try to auto-select repo by matching "owner/repo" in remotes
                        let search_pattern = format!("{}/{}", github.pr.owner, github.pr.repo);
                        if let Some(matched_repo) = state.linked_repos.iter().find(|r| {
                            r.remotes
                                .iter()
                                .any(|remote| remote.contains(&search_pattern))
                        }) {
                            state.selected_repo_id = Some(matched_repo.id.clone());
                        }
                    }
                }
                Err(err) => {
                    state.generate_preview = None;
                    state.generation_error = Some(err);
                }
            }
            Vec::new()
        }
        GenMsg::InputResolved(result) => match *result {
            Ok(payload) => {
                state.generate_preview = Some(payload.preview);
                state.selected_review_id = Some(payload.run_context.review_id.clone());
                state.selected_run_id = Some(payload.run_context.run_id.clone());
                vec![Command::StartGeneration {
                    run_context: Box::new(payload.run_context),
                    selected_agent_id: state.selected_agent.id.clone(),
                }]
            }
            Err(err) => {
                state.is_generating = false;
                state.generation_error = Some(err);
                Vec::new()
            }
        },
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
    state.reviews = payload.reviews;
    state.runs = payload.runs;
    state.all_tasks = payload.tasks;
    state.review_error = None;
    state.cached_unified_diff = None;

    if let Some(selected) = &state.selected_review_id
        && !state.reviews.iter().any(|r| &r.id == selected)
    {
        state.selected_review_id = None;
    }

    if state.selected_review_id.is_none() {
        state.selected_review_id = state.reviews.first().map(|r| r.id.clone());
    }

    if let Some(selected_review_id) = &state.selected_review_id {
        let default_run_id = state
            .reviews
            .iter()
            .find(|r| &r.id == selected_review_id)
            .and_then(|r| r.active_run_id.clone());

        let run_in_review = state.selected_run_id.as_ref().is_some_and(|run_id| {
            state
                .runs
                .iter()
                .any(|run| &run.id == run_id && &run.review_id == selected_review_id)
        });

        if !run_in_review {
            state.selected_run_id = default_run_id;
        }
    } else {
        state.selected_run_id = None;
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
        state.task_line_notes.clear();
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
    state.task_line_notes.clear();
    state.cached_unified_diff = None;
    state.current_note = Some(String::new());
    vec![Command::LoadTaskNote { task_id }]
}

fn select_default_task_for_current_run(state: &mut AppState) -> Vec<Command> {
    let current_tasks = state.tasks();
    let Some(next_open) = current_tasks
        .iter()
        .find(|t| matches!(t.status, TaskStatus::Pending | TaskStatus::InProgress))
    else {
        return Vec::new();
    };

    state.selected_task_id = Some(next_open.id.clone());
    state.current_line_note = None;
    state.task_line_notes.clear();
    state.current_note = Some(String::new());
    vec![Command::LoadTaskNote {
        task_id: next_open.id.clone(),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ReviewTask, TaskStats, TaskStatus};
    use crate::ui::app::{GenMsg, GenResultPayload, SelectedAgent};

    fn pending_task(id: &str, run_id: &str) -> ReviewTask {
        ReviewTask {
            id: id.to_string(),
            run_id: run_id.to_string(),
            title: "Task".into(),
            description: "Desc".into(),
            files: vec![],
            stats: TaskStats::default(),
            diff_refs: vec![],
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
                [Command::ResolveGenerateInput { selected_agent_id, .. }]
                if selected_agent_id == "agent-1"
            ),
            "expected ResolveGenerateInput command"
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
            Action::Async(AsyncAction::GenerationMessage(Box::new(GenMsg::Done(Ok(
                GenResultPayload {
                    messages: vec![],
                    thoughts: vec![],
                    logs: vec![],
                },
            ))))),
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
        let review = crate::domain::Review {
            id: "rev1".into(),
            title: "Review".into(),
            summary: None,
            source: crate::domain::ReviewSource::DiffPaste {
                diff_hash: "h".into(),
            },
            active_run_id: Some("run1".into()),
            created_at: "now".into(),
            updated_at: "now".into(),
        };
        let task = pending_task("t1", "run1");
        let mut state = AppState::default();

        let commands = reduce(
            &mut state,
            Action::Async(AsyncAction::ReviewDataLoaded {
                reason: ReviewDataRefreshReason::Manual,
                result: Ok(ReviewDataPayload {
                    reviews: vec![review.clone()],
                    runs: vec![],
                    tasks: vec![task.clone()],
                }),
            }),
        );

        assert_eq!(state.selected_review_id.as_deref(), Some("rev1"));
        assert_eq!(state.selected_run_id.as_deref(), Some("run1"));
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
    fn review_data_loaded_without_reviews_keeps_selection_empty() {
        let mut state = AppState::default();
        let commands = reduce(
            &mut state,
            Action::Async(AsyncAction::ReviewDataLoaded {
                reason: ReviewDataRefreshReason::Manual,
                result: Ok(ReviewDataPayload {
                    reviews: vec![],
                    runs: vec![],
                    tasks: vec![],
                }),
            }),
        );

        assert!(state.selected_review_id.is_none());
        assert!(state.selected_run_id.is_none());
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
                    reviews: vec![],
                    runs: vec![],
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
