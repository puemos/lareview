pub mod async_handler;
pub mod generate;
pub mod navigation;
pub mod review;
pub mod settings;

use super::super::state::AppState;
use super::action::Action;
use super::command::Command;

pub fn reduce(state: &mut AppState, action: Action) -> Vec<Command> {
    match action {
        Action::Navigation(action) => navigation::reduce(&mut state.ui, &mut state.session, action),
        Action::Generate(action) => generate::reduce(&mut state.ui, &mut state.session, action),
        Action::Review(action) => review::reduce(state, action),
        Action::Settings(action) => settings::reduce(&mut state.ui, &mut state.session, action),
        Action::Async(action) => async_handler::reduce(state, action),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        Comment, ReviewTask, TaskStats, TaskStatus, Thread, ThreadImpact, ThreadStatus,
    };
    use crate::ui::app::state::{AppState, AppView, SessionState, UiState};
    use crate::ui::app::store::action::{
        AsyncAction, GenerateAction, NavigationAction, ReviewAction, ReviewDataPayload,
        ReviewThreadsPayload,
    };
    use crate::ui::app::store::command::{Command, D2Command, ReviewDataRefreshReason};
    use crate::ui::app::{GenMsg, GenResultPayload, SelectedAgent, SettingsAction};

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
            session: SessionState {
                diff_text: "diff --git a b".into(),
                selected_agent: SelectedAgent::new("agent-1"),
                ..Default::default()
            },
            ..Default::default()
        };

        let commands = reduce(&mut state, Action::Generate(GenerateAction::RunRequested));

        assert!(state.session.is_generating);
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
            session: SessionState {
                is_generating: true,
                ..Default::default()
            },
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

        assert!(!state.session.is_generating);
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

        assert_eq!(state.ui.selected_review_id.as_deref(), Some("rev1"));
        assert_eq!(state.ui.selected_run_id.as_deref(), Some("run1"));
        assert_eq!(state.ui.selected_task_id.as_deref(), Some("t1"));
        assert!(
            matches!(
                commands.as_slice(),
                [Command::LoadReviewThreads { review_id }]
                if review_id == "rev1"
            ),
            "expected thread load for selected review"
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
        assert!(state.ui.review_error.is_none());
    }

    #[test]
    fn settings_install_only_when_allowed() {
        let mut state = AppState {
            ui: UiState {
                allow_d2_install: false,
                is_d2_installing: false,
                ..Default::default()
            },
            ..Default::default()
        };

        let none = reduce(
            &mut state,
            Action::Settings(SettingsAction::RequestD2Install),
        );
        assert!(none.is_empty());
        assert!(!state.ui.is_d2_installing);

        state.ui.allow_d2_install = true;
        let commands = reduce(
            &mut state,
            Action::Settings(SettingsAction::RequestD2Install),
        );
        assert!(state.ui.is_d2_installing);
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

        assert_eq!(state.ui.current_view, AppView::Review);
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

        assert!(state.ui.selected_review_id.is_none());
        assert!(state.ui.selected_run_id.is_none());
        assert!(commands.is_empty(), "no note load without tasks");
    }

    #[test]
    fn review_data_after_generation_with_no_tasks_stays_on_generate() {
        let mut state = AppState {
            ui: UiState {
                current_view: AppView::Generate,
                ..Default::default()
            },
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
        assert_eq!(state.ui.current_view, AppView::Generate);
        assert_eq!(
            state.session.generation_error.as_deref(),
            Some("No tasks generated")
        );
    }

    #[test]
    fn create_thread_comment_emits_command() {
        let mut state = AppState {
            ui: UiState {
                selected_review_id: Some("review-1".into()),
                ..Default::default()
            },
            ..AppState::default()
        };

        let commands = reduce(
            &mut state,
            Action::Review(ReviewAction::CreateThreadComment {
                task_id: "task-1".into(),
                thread_id: None,
                file_path: Some("src/main.rs".into()),
                line_number: Some(42),
                title: Some("Title".into()),
                body: "Hello".into(),
            }),
        );

        assert!(
            matches!(
                commands.as_slice(),
                [Command::CreateThreadComment { review_id, task_id, .. }]
                if review_id == "review-1" && task_id == "task-1"
            ),
            "expected CreateThreadComment command"
        );
    }

    #[test]
    fn update_thread_status_emits_command() {
        let mut state = AppState::default();
        let commands = reduce(
            &mut state,
            Action::Review(ReviewAction::UpdateThreadStatus {
                thread_id: "thread-1".into(),
                status: ThreadStatus::Wip,
            }),
        );

        assert!(
            matches!(
                commands.as_slice(),
                [Command::UpdateThreadStatus { thread_id, status }]
                if thread_id == "thread-1" && *status == ThreadStatus::Wip
            ),
            "expected UpdateThreadStatus command"
        );
    }

    #[test]
    fn review_threads_loaded_updates_state_for_selected_review() {
        let mut state = AppState {
            ui: UiState {
                selected_review_id: Some("review-1".into()),
                ..Default::default()
            },
            ..AppState::default()
        };

        let thread = Thread {
            id: "thread-1".into(),
            review_id: "review-1".into(),
            task_id: Some("task-1".into()),
            title: "Thread".into(),
            status: ThreadStatus::Todo,
            impact: ThreadImpact::Nitpick,
            anchor: None,
            author: "User".into(),
            created_at: "2025-01-01T00:00:00Z".into(),
            updated_at: "2025-01-01T00:00:00Z".into(),
        };

        let comment = Comment {
            id: "comment-1".into(),
            thread_id: "thread-1".into(),
            author: "User".into(),
            body: "Hello".into(),
            parent_id: None,
            created_at: "2025-01-01T00:00:00Z".into(),
            updated_at: "2025-01-01T00:00:00Z".into(),
        };

        let mut comments = std::collections::HashMap::new();
        comments.insert("thread-1".into(), vec![comment]);

        let commands = reduce(
            &mut state,
            Action::Async(AsyncAction::ReviewThreadsLoaded(Ok(ReviewThreadsPayload {
                review_id: "review-1".into(),
                threads: vec![thread],
                comments,
            }))),
        );

        assert!(commands.is_empty());
        assert_eq!(state.domain.threads.len(), 1);
        assert_eq!(
            state
                .domain
                .thread_comments
                .get("thread-1")
                .map(|items| items.len()),
            Some(1)
        );
    }
}
