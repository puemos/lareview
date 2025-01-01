use super::super::super::state::{AppState, AppView};
use super::super::action::AsyncAction;
use super::super::command::{Command, ReviewDataRefreshReason};
use super::generate::reduce_msg;
use super::review::apply_review_data;

pub fn reduce(state: &mut AppState, action: AsyncAction) -> Vec<Command> {
    match action {
        AsyncAction::GenerationMessage(msg) => {
            reduce_msg(&mut state.ui, &mut state.session, &mut state.domain, *msg)
        }
        AsyncAction::GhStatusLoaded(result) => {
            state.session.is_gh_status_checking = false;
            match result {
                Ok(status) => {
                    state.session.gh_status = Some(status);
                    state.session.gh_status_error = None;
                }
                Err(err) => {
                    state.session.gh_status = None;
                    state.session.gh_status_error = Some(err);
                }
            }
            Vec::new()
        }
        AsyncAction::ReviewDataLoaded { reason, result } => match result {
            Ok(payload) => {
                let commands = apply_review_data(state, payload);

                if matches!(reason, ReviewDataRefreshReason::AfterGeneration) {
                    if state.tasks().is_empty() {
                        state.ui.current_view = AppView::Generate;
                        state.session.generation_error = Some("No tasks generated".to_string());
                    } else {
                        state.ui.current_view = AppView::Review;
                        state.session.generation_error = None;
                    }
                }
                commands
            }
            Err(err) => {
                state.ui.review_error = Some(err);
                Vec::new()
            }
        },
        AsyncAction::ReviewFeedbacksLoaded(result) => {
            match result {
                Ok(payload) => {
                    if state.ui.selected_review_id.as_deref() == Some(payload.review_id.as_str()) {
                        state.domain.feedbacks = payload.feedbacks;
                        state.domain.feedback_comments = payload.comments;
                    }
                }
                Err(err) => {
                    state.ui.review_error = Some(err);
                }
            }
            Vec::new()
        }
        AsyncAction::ReviewFeedbackLinksLoaded(result) => {
            match result {
                Ok(payload) => {
                    if state.ui.selected_review_id.as_deref() == Some(payload.review_id.as_str()) {
                        state.domain.feedback_links = payload.links;
                    }
                }
                Err(err) => {
                    state.ui.review_error = Some(err);
                }
            }
            Vec::new()
        }
        AsyncAction::ExportPreviewGenerated(result) => {
            state.ui.is_exporting = false;
            match result {
                Ok(res) => {
                    state.ui.export_preview = Some(res.markdown);
                    state.ui.review_error = None;
                }
                Err(err) => {
                    state.ui.export_preview = None;
                    state.ui.review_error = Some(err);
                }
            }
            Vec::new()
        }
        AsyncAction::ExportFinished(result) => {
            state.ui.is_exporting = false;
            if let Err(err) = result {
                state.ui.review_error = Some(err);
            } else {
                state.ui.export_copy_success = true;
                state.ui.export_save_success = true;
                state.ui.export_copy_shown_frames = 0;
                state.ui.export_save_shown_frames = 0;
            }
            Vec::new()
        }
        AsyncAction::TaskStatusSaved(result) => {
            if let Err(err) = result {
                state.ui.review_error = Some(err);
            } else {
                return vec![Command::RefreshReviewData {
                    reason: ReviewDataRefreshReason::AfterStatusChange,
                }];
            }
            Vec::new()
        }
        AsyncAction::FeedbackCommentSaved(result) => {
            if let Err(err) = result {
                state.ui.review_error = Some(err);
            }
            Vec::new()
        }
        AsyncAction::ReviewDeleted(result) => {
            if let Err(err) = result {
                state.ui.review_error = Some(err);
            } else {
                state.ui.selected_review_id = None;
                state.ui.selected_run_id = None;
                state.ui.selected_task_id = None;
                return vec![Command::RefreshReviewData {
                    reason: ReviewDataRefreshReason::AfterReviewDelete,
                }];
            }
            Vec::new()
        }
        AsyncAction::D2InstallOutput(output) => {
            state.ui.is_d2_installing = false;
            state.ui.d2_install_output = output;
            Vec::new()
        }
        AsyncAction::D2InstallComplete => {
            state.ui.is_d2_installing = false;
            Vec::new()
        }
        AsyncAction::FeedbackPushed(result) => {
            state.ui.push_feedback_pending = None;
            match result {
                Ok(link) => {
                    state.ui.show_push_feedback_modal = None;
                    state.ui.push_feedback_error = None;
                    state
                        .domain
                        .feedback_links
                        .insert(link.feedback_id.clone(), link);
                    if let Some(review_id) = state.ui.selected_review_id.clone() {
                        return vec![
                            Command::LoadReviewFeedbacks {
                                review_id: review_id.clone(),
                            },
                            Command::LoadFeedbackLinks { review_id },
                        ];
                    }
                }
                Err(err) => {
                    state.ui.push_feedback_error = Some(err);
                }
            }
            Vec::new()
        }
        AsyncAction::NewRepoPicked(repo) => {
            vec![Command::SaveRepo { repo }]
        }
        AsyncAction::RepoDeleted(result) => {
            if let Err(err) = result {
                state.ui.review_error = Some(err);
            } else if let Ok(repo_id) = result {
                state.domain.linked_repos.retain(|r| r.id != repo_id);
                return vec![Command::RefreshReviewData {
                    reason: ReviewDataRefreshReason::Manual,
                }];
            }
            Vec::new()
        }
        AsyncAction::ReposLoaded(result) => {
            match result {
                Ok(repos) => {
                    state.domain.linked_repos = repos;
                }
                Err(err) => {
                    state.ui.review_error = Some(err);
                }
            }
            Vec::new()
        }
        AsyncAction::RepoSaved(result) => {
            match result {
                Ok(repo) => {
                    if let Some(idx) = state
                        .domain
                        .linked_repos
                        .iter()
                        .position(|r| r.id == repo.id)
                    {
                        state.domain.linked_repos[idx] = repo;
                    } else {
                        state.domain.linked_repos.push(repo);
                    }
                }
                Err(err) => {
                    state.ui.review_error = Some(err);
                }
            }
            Vec::new()
        }
    }
}
