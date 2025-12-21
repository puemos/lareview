use super::super::super::state::{AppState, AppView};
use super::super::action::AsyncAction;
use super::super::command::{Command, ReviewDataRefreshReason};
use super::generate::reduce_msg;
use super::review::apply_review_data;

pub fn reduce(state: &mut AppState, action: AsyncAction) -> Vec<Command> {
    match action {
        AsyncAction::GenerationMessage(msg) => reduce_msg(state, *msg),
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
        AsyncAction::ReviewThreadsLoaded(result) => {
            match result {
                Ok(payload) => {
                    if state.selected_review_id.as_deref() == Some(payload.review_id.as_str()) {
                        state.threads = payload.threads;
                        state.thread_comments = payload.comments;
                    }
                }
                Err(err) => {
                    state.review_error = Some(err);
                }
            }
            Vec::new()
        }
        AsyncAction::ExportPreviewGenerated(result) => {
            state.is_exporting = false;
            match result {
                Ok(res) => {
                    state.export_preview = Some(res.markdown);
                    state.review_error = None;
                }
                Err(err) => {
                    state.export_preview = None;
                    state.review_error = Some(err);
                }
            }
            Vec::new()
        }
        AsyncAction::ExportFinished(result) => {
            state.is_exporting = false;
            if let Err(err) = result {
                state.review_error = Some(err);
            }
            Vec::new()
        }
        AsyncAction::TaskStatusSaved(result) => {
            if let Err(err) = result {
                state.review_error = Some(err);
            } else {
                return vec![Command::RefreshReviewData {
                    reason: ReviewDataRefreshReason::AfterStatusChange,
                }];
            }
            Vec::new()
        }
        AsyncAction::ThreadCommentSaved(result) => {
            if let Err(err) = result {
                state.review_error = Some(err);
            }
            Vec::new()
        }
        AsyncAction::ReviewDeleted(result) => {
            if let Err(err) = result {
                state.review_error = Some(err);
            } else {
                state.selected_review_id = None;
                state.selected_run_id = None;
                state.selected_task_id = None;
                return vec![Command::RefreshReviewData {
                    reason: ReviewDataRefreshReason::AfterReviewDelete,
                }];
            }
            Vec::new()
        }
        AsyncAction::D2InstallOutput(output) => {
            state.is_d2_installing = false;
            state.d2_install_output = output;
            Vec::new()
        }
        AsyncAction::D2InstallComplete => {
            state.is_d2_installing = false;
            Vec::new()
        }
        AsyncAction::NewRepoPicked(repo) => {
            vec![Command::SaveRepo { repo }]
        }
        AsyncAction::RepoDeleted(result) => {
            if let Err(err) = result {
                state.review_error = Some(err);
            } else {
                return vec![Command::RefreshReviewData {
                    reason: ReviewDataRefreshReason::Manual,
                }];
            }
            Vec::new()
        }
        AsyncAction::ReposLoaded(result) => {
            match result {
                Ok(repos) => {
                    state.linked_repos = repos;
                }
                Err(err) => {
                    state.review_error = Some(err);
                }
            }
            Vec::new()
        }
        AsyncAction::RepoSaved(result) => {
            match result {
                Ok(repo) => {
                    if let Some(idx) = state.linked_repos.iter().position(|r| r.id == repo.id) {
                        state.linked_repos[idx] = repo;
                    } else {
                        state.linked_repos.push(repo);
                    }
                }
                Err(err) => {
                    state.review_error = Some(err);
                }
            }
            Vec::new()
        }
    }
}
