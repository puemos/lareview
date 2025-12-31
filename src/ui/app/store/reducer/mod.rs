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
    use crate::domain::{Comment, Feedback, FeedbackImpact, ReviewStatus, ReviewTask, TaskStats};
    use crate::ui::app::state::{AppState, AppView, SessionState, UiState};
    use crate::ui::app::store::action::{
        AsyncAction, GenerateAction, NavigationAction, ReviewAction, ReviewDataPayload,
        ReviewFeedbacksPayload,
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
            status: ReviewStatus::Todo,
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
                [
                    Command::LoadReviewFeedbacks { review_id },
                    Command::LoadFeedbackLinks { review_id: review_id_links }
                ] if review_id == "rev1" && review_id_links == "rev1"
            ),
            "expected feedback load for selected review"
        );
    }

    #[test]
    fn update_task_status_emits_command() {
        let mut state = AppState::default();
        let commands = reduce(
            &mut state,
            Action::Review(ReviewAction::UpdateTaskStatus {
                task_id: "t1".into(),
                status: ReviewStatus::Done,
            }),
        );

        assert!(
            matches!(
                commands.as_slice(),
                [Command::UpdateTaskStatus { task_id, status: ReviewStatus::Done }]
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
    fn create_feedback_comment_emits_command() {
        let mut state = AppState {
            ui: UiState {
                selected_review_id: Some("review-1".into()),
                ..Default::default()
            },
            ..AppState::default()
        };

        let commands = reduce(
            &mut state,
            Action::Review(ReviewAction::CreateFeedbackComment {
                task_id: "task-1".into(),
                feedback_id: None,
                file_path: Some("src/main.rs".into()),
                line_number: Some(42),
                side: None,
                title: Some("Title".into()),
                body: "Hello".into(),
            }),
        );

        assert!(
            matches!(
                commands.as_slice(),
                [Command::CreateFeedbackComment { review_id, task_id, .. }]
                if review_id == "review-1" && task_id == "task-1"
            ),
            "expected CreateFeedbackComment command"
        );
    }

    #[test]
    fn update_feedback_status_emits_command() {
        let mut state = AppState::default();
        let commands = reduce(
            &mut state,
            Action::Review(ReviewAction::UpdateFeedbackStatus {
                feedback_id: "feedback-1".into(),
                status: ReviewStatus::InProgress,
            }),
        );

        assert!(
            matches!(
                commands.as_slice(),
                [Command::UpdateFeedbackStatus { feedback_id, status }]
                if feedback_id == "feedback-1" && *status == ReviewStatus::InProgress
            ),
            "expected UpdateFeedbackStatus command"
        );
    }

    #[test]
    fn review_feedbacks_loaded_updates_state_for_selected_review() {
        let mut state = AppState {
            ui: UiState {
                selected_review_id: Some("review-1".into()),
                ..Default::default()
            },
            ..AppState::default()
        };

        let feedback = Feedback {
            id: "feedback-1".into(),
            review_id: "review-1".into(),
            task_id: Some("task-1".into()),
            title: "Feedback".into(),
            status: ReviewStatus::Todo,
            impact: FeedbackImpact::Nitpick,
            anchor: None,
            author: "User".into(),
            created_at: "2025-01-01T00:00:00Z".into(),
            updated_at: "2025-01-01T00:00:00Z".into(),
        };

        let comment = Comment {
            id: "comment-1".into(),
            feedback_id: "thread-1".into(),
            author: "User".into(),
            body: "Hello".into(),
            parent_id: None,
            created_at: "2025-01-01T00:00:00Z".into(),
            updated_at: "2025-01-01T00:00:00Z".into(),
        };

        let mut comments = std::collections::HashMap::new();
        comments.insert("feedback-1".into(), vec![comment]);

        let commands = reduce(
            &mut state,
            Action::Async(AsyncAction::ReviewFeedbacksLoaded(Ok(
                ReviewFeedbacksPayload {
                    review_id: "review-1".into(),
                    feedbacks: vec![feedback],
                    comments,
                },
            ))),
        );

        assert!(commands.is_empty());
        assert_eq!(state.domain.feedbacks.len(), 1);
        assert_eq!(
            state
                .domain
                .feedback_comments
                .get("feedback-1")
                .map(|items| items.len()),
            Some(1)
        );
    }

    #[test]
    fn test_dismiss_requirements_emits_save_config() {
        let mut state = AppState::default();
        state.ui.show_requirements_modal = true;

        let commands = reduce(
            &mut state,
            Action::Settings(SettingsAction::DismissRequirements),
        );

        assert!(!state.ui.show_requirements_modal);
        assert!(state.ui.has_seen_requirements);
        assert!(matches!(
            commands.as_slice(),
            [Command::SaveAppConfigFull {
                has_seen_requirements: true,
                ..
            }]
        ));
    }

    #[test]
    fn test_navigation_to_generate_resets_session() {
        let mut state = AppState {
            session: SessionState {
                is_generating: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let _ = reduce(
            &mut state,
            Action::Navigation(NavigationAction::SwitchTo(AppView::Generate)),
        );

        assert_eq!(state.ui.current_view, AppView::Generate);
    }

    #[test]
    fn test_review_action_select_review() {
        let mut state = AppState::default();
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
        state.domain.reviews.push(review);

        let commands = reduce(
            &mut state,
            Action::Review(ReviewAction::SelectReview {
                review_id: "rev1".into(),
            }),
        );

        assert_eq!(state.ui.selected_review_id.as_deref(), Some("rev1"));
        assert_eq!(state.ui.selected_run_id.as_deref(), Some("run1"));
        assert!(
            commands
                .iter()
                .any(|c| matches!(c, Command::LoadReviewFeedbacks { .. }))
        );
    }

    #[test]
    fn test_review_action_delete_review() {
        let mut state = AppState::default();
        state.ui.selected_review_id = Some("rev1".into());

        let commands = reduce(
            &mut state,
            Action::Review(ReviewAction::DeleteReview("rev1".into())),
        );

        assert!(state.ui.selected_review_id.is_none());
        assert!(matches!(
            commands.as_slice(),
            [Command::DeleteReview { .. }]
        ));
    }

    #[test]
    fn test_navigate_to_feedback() {
        let mut state = AppState::default();
        let feedback = Feedback {
            id: "t1".into(),
            review_id: "r1".into(),
            task_id: Some("task1".into()),
            title: "Title".into(),
            status: ReviewStatus::Todo,
            impact: FeedbackImpact::Nitpick,
            anchor: None,
            author: "User".into(),
            created_at: "now".into(),
            updated_at: "now".into(),
        };

        reduce(
            &mut state,
            Action::Review(ReviewAction::NavigateToFeedback(feedback)),
        );

        assert_eq!(state.ui.selected_task_id.as_deref(), Some("task1"));
        assert_eq!(state.ui.current_view, AppView::Review);
        assert!(state.ui.active_feedback.is_some());
    }

    #[test]
    fn test_gen_msg_input_resolved() {
        let mut state = AppState::default();
        let payload = crate::ui::app::messages::GenerateResolvedPayload {
            run_context: crate::infra::acp::RunContext {
                review_id: "rev1".into(),
                run_id: "run1".into(),
                agent_id: "agent1".into(),
                input_ref: "ref1".into(),
                diff_text: "diff".into(),
                diff_hash: "hash".into(),
                source: crate::domain::ReviewSource::DiffPaste {
                    diff_hash: "hash".into(),
                },
                initial_title: None,
                created_at: None,
            },
            preview: crate::ui::app::state::GeneratePreview {
                diff_text: "diff".into(),
                github: None,
            },
        };

        let commands = reduce(
            &mut state,
            Action::Async(AsyncAction::GenerationMessage(Box::new(
                GenMsg::InputResolved(Box::new(Ok(payload))),
            ))),
        );

        assert_eq!(state.ui.selected_review_id.as_deref(), Some("rev1"));
        assert!(
            commands
                .iter()
                .any(|c| matches!(c, Command::StartGeneration { .. }))
        );
    }

    #[test]
    fn test_gen_msg_progress() {
        let mut state = AppState::default();
        let evt = crate::infra::acp::ProgressEvent::TaskStarted("t1".into());

        let commands = reduce(
            &mut state,
            Action::Async(AsyncAction::GenerationMessage(Box::new(GenMsg::Progress(
                Box::new(evt),
            )))),
        );

        assert!(commands.iter().any(|c| matches!(
            c,
            Command::RefreshReviewData {
                reason: ReviewDataRefreshReason::Incremental
            }
        )));
        assert_eq!(state.session.agent_timeline.len(), 1);
    }

    #[test]
    fn test_navigation_same_view() {
        let mut state = AppState {
            ui: UiState {
                current_view: AppView::Generate,
                ..Default::default()
            },
            ..Default::default()
        };
        let commands = reduce(
            &mut state,
            Action::Navigation(NavigationAction::SwitchTo(AppView::Generate)),
        );
        assert!(commands.is_empty());
    }

    #[test]
    fn test_generate_action_reset() {
        let mut state = AppState {
            ui: UiState {
                current_view: AppView::Review,
                ..Default::default()
            },
            session: SessionState {
                diff_text: "some diff".into(),
                is_generating: true,
                generation_error: Some("Error".into()),
                ..Default::default()
            },
            ..Default::default()
        };

        let commands = reduce(&mut state, Action::Generate(GenerateAction::Reset));

        assert!(state.session.diff_text.is_empty());
        assert!(!state.session.is_generating);
        assert!(state.session.generation_error.is_none());
        assert_eq!(state.ui.current_view, AppView::Generate);
        assert!(matches!(commands.as_slice(), [Command::AbortGeneration]));
    }

    #[test]
    fn test_generate_action_update_diff_text() {
        let mut state = AppState::default();
        reduce(
            &mut state,
            Action::Generate(GenerateAction::UpdateDiffText("new diff".into())),
        );
        assert_eq!(state.session.diff_text, "new diff");
    }

    #[test]
    fn test_generate_action_select_repo() {
        let mut state = AppState::default();
        reduce(
            &mut state,
            Action::Generate(GenerateAction::SelectRepo(Some("repo1".into()))),
        );
        assert_eq!(state.ui.selected_repo_id.as_deref(), Some("repo1"));
    }

    #[test]
    fn test_generate_action_clear_timeline() {
        let mut state = AppState::default();
        state
            .session
            .agent_timeline
            .push(crate::ui::app::TimelineItem {
                seq: 1,
                stream_key: None,
                content: crate::ui::app::TimelineContent::LocalLog("log".into()),
            });
        reduce(&mut state, Action::Generate(GenerateAction::ClearTimeline));
        assert!(state.session.agent_timeline.is_empty());
    }

    #[test]
    fn test_review_action_clear_selection() {
        let mut state = AppState {
            ui: UiState {
                selected_task_id: Some("t1".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        reduce(&mut state, Action::Review(ReviewAction::ClearSelection));
        assert!(state.ui.selected_task_id.is_none());
    }

    #[test]
    fn test_review_action_close_feedback() {
        let mut state = AppState {
            ui: UiState {
                active_feedback: Some(crate::ui::app::FeedbackContext {
                    feedback_id: None,
                    task_id: "t1".into(),
                    file_path: None,
                    line_number: None,
                    side: None,
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        reduce(&mut state, Action::Review(ReviewAction::CloseFeedback));
        assert!(state.ui.active_feedback.is_none());
    }

    #[test]
    fn test_generate_action_update_diff_text_triggers_preview() {
        let mut state = AppState::default();
        let pr_url = "https://github.com/owner/repo/pull/123";
        let commands = reduce(
            &mut state,
            Action::Generate(GenerateAction::UpdateDiffText(pr_url.into())),
        );

        assert_eq!(state.session.diff_text, pr_url);
        assert!(state.session.is_preview_fetching);
        assert_eq!(
            state.session.last_preview_input_ref.as_deref(),
            Some(pr_url)
        );
        assert!(
            matches!(
                commands.as_slice(),
                [Command::FetchPrContextPreview { input_ref }]
                if input_ref == pr_url
            ),
            "expected fetch context command"
        );
    }

    #[test]
    fn test_generate_action_fetch_pr_context_valid() {
        let mut state = AppState::default();
        let ref_str = "owner/repo#123";
        let commands = reduce(
            &mut state,
            Action::Generate(GenerateAction::FetchPrContext(ref_str.into())),
        );

        assert!(state.session.is_preview_fetching);
        assert_eq!(
            state.session.last_preview_input_ref.as_deref(),
            Some(ref_str)
        );
        assert!(matches!(
            commands.as_slice(),
            [Command::FetchPrContextPreview { input_ref }]
            if input_ref == ref_str
        ));
    }

    #[test]
    fn test_generate_action_fetch_pr_context_ignored() {
        let mut state = AppState {
            session: SessionState {
                last_preview_input_ref: Some("owner/repo#123".into()),
                generate_preview: Some(crate::ui::app::state::GeneratePreview {
                    diff_text: "".into(),
                    github: None,
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        let commands = reduce(
            &mut state,
            Action::Generate(GenerateAction::FetchPrContext("owner/repo#123".into())),
        );
        assert!(commands.is_empty());
    }

    #[test]
    fn test_gen_msg_preview_resolved_success_auto_repo_select() {
        let mut state = AppState::default();
        state.session.diff_text = "owner/repo#123".into();
        state.session.is_preview_fetching = true;

        let linked_repo = crate::domain::LinkedRepo {
            id: "repo-1".into(),
            name: "repo".into(),
            path: "/path".into(),
            remotes: vec!["https://github.com/owner/repo.git".into()],
            created_at: "now".into(),
        };
        state.domain.linked_repos.push(linked_repo);

        let pr_ref = crate::infra::vcs::github::GitHubPrRef {
            owner: "owner".into(),
            repo: "repo".into(),
            number: 123,
            url: "url".into(),
        };
        let meta = crate::infra::vcs::github::GitHubPrMetadata {
            title: "Title".into(),
            url: "url".into(),
            head_sha: None,
            base_sha: None,
        };
        let preview = crate::ui::app::state::GeneratePreview {
            diff_text: "diff".into(),
            github: Some(crate::ui::app::state::GitHubPreview { pr: pr_ref, meta }),
        };

        let commands = reduce(
            &mut state,
            Action::Async(AsyncAction::GenerationMessage(Box::new(
                GenMsg::PreviewResolved {
                    input_ref: "owner/repo#123".into(),
                    result: Ok(preview),
                },
            ))),
        );

        assert!(!state.session.is_preview_fetching);
        assert!(state.session.generate_preview.is_some());
        assert_eq!(state.ui.selected_repo_id.as_deref(), Some("repo-1"));
        assert!(commands.is_empty());
    }

    #[test]
    fn test_gen_msg_preview_resolved_error() {
        let mut state = AppState::default();
        state.session.diff_text = "ref".into();
        state.session.is_preview_fetching = true;

        let _commands = reduce(
            &mut state,
            Action::Async(AsyncAction::GenerationMessage(Box::new(
                GenMsg::PreviewResolved {
                    input_ref: "ref".into(),
                    result: Err("Failed".to_string()),
                },
            ))),
        );

        assert!(!state.session.is_preview_fetching);
        assert!(state.session.generate_preview.is_none());
        assert_eq!(state.session.generation_error.as_deref(), Some("Failed"));
    }

    #[test]
    fn test_async_action_gh_status_loaded() {
        let mut state = AppState::default();
        state.session.is_gh_status_checking = true;

        reduce(
            &mut state,
            Action::Async(AsyncAction::GhStatusLoaded(Ok(
                crate::ui::app::GhStatusPayload {
                    gh_path: "/bin/gh".into(),
                    login: Some("user".into()),
                },
            ))),
        );

        assert!(!state.session.is_gh_status_checking);
        assert!(state.session.gh_status.is_some());
        assert!(state.session.gh_status_error.is_none());

        reduce(
            &mut state,
            Action::Async(AsyncAction::GhStatusLoaded(Err("Error".to_string()))),
        );
        assert!(state.session.gh_status.is_none());
        assert_eq!(state.session.gh_status_error.as_deref(), Some("Error"));
    }

    #[test]
    fn test_async_action_export_preview_generated() {
        let mut state = AppState::default();
        state.ui.is_exporting = true;

        reduce(
            &mut state,
            Action::Async(AsyncAction::ExportPreviewGenerated(Ok(
                crate::application::review::export::ExportResult {
                    markdown: "MD".into(),
                    assets: std::collections::HashMap::new(),
                },
            ))),
        );

        assert!(!state.ui.is_exporting);
        assert_eq!(state.ui.export_preview.as_deref(), Some("MD"));
        assert!(state.ui.review_error.is_none());

        reduce(
            &mut state,
            Action::Async(AsyncAction::ExportPreviewGenerated(
                Err("Error".to_string()),
            )),
        );
        assert!(state.ui.export_preview.is_none());
        assert_eq!(state.ui.review_error.as_deref(), Some("Error"));
    }

    #[test]
    fn test_async_action_task_status_saved() {
        let mut state = AppState::default();

        let commands = reduce(
            &mut state,
            Action::Async(AsyncAction::TaskStatusSaved(Ok(()))),
        );
        assert!(matches!(
            commands.as_slice(),
            [Command::RefreshReviewData {
                reason: ReviewDataRefreshReason::AfterStatusChange
            }]
        ));

        reduce(
            &mut state,
            Action::Async(AsyncAction::TaskStatusSaved(Err("Error".to_string()))),
        );
        assert_eq!(state.ui.review_error.as_deref(), Some("Error"));
    }

    #[test]
    fn test_async_action_repo_deleted() {
        let mut state = AppState::default();
        let repo = crate::domain::LinkedRepo {
            id: "r1".into(),
            name: "n".into(),
            path: "p".into(),
            remotes: vec![],
            created_at: "t".into(),
        };
        state.domain.linked_repos.push(repo);

        let commands = reduce(
            &mut state,
            Action::Async(AsyncAction::RepoDeleted(Ok("r1".into()))),
        );

        assert!(state.domain.linked_repos.is_empty());
        assert!(matches!(
            commands.as_slice(),
            [Command::RefreshReviewData {
                reason: ReviewDataRefreshReason::Manual
            }]
        ));

        reduce(
            &mut state,
            Action::Async(AsyncAction::RepoDeleted(Err("Error".to_string()))),
        );
        assert_eq!(state.ui.review_error.as_deref(), Some("Error"));
    }
}
