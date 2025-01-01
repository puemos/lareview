use super::super::super::state::{AppState, AppView};
use super::super::action::{ReviewAction, ReviewDataPayload};
use super::super::command::Command;
use crate::domain::{ReviewStatus, TaskId};
use chrono::Utc;
use std::collections::HashSet;
use std::path::Path; // Keep Path as it's used
// The user's diff suggested adding HashMap and removing Path, but Path is used.
// The instruction was "remove unused HashSet import" which is not present.
// I will keep Path and not add HashMap as it's not used.

pub fn reduce(state: &mut AppState, action: ReviewAction) -> Vec<Command> {
    match action {
        ReviewAction::NavigateToFeedback(feedback) => {
            // 1. Select the task (this usually clears active_feedback, so we do it first)
            if let Some(task_id) = feedback.task_id.clone() {
                select_task(state, task_id.clone());

                // 2. Set the active feedback context
                let file_path = feedback.anchor.as_ref().and_then(|a| a.file_path.clone());
                let line_number = feedback.anchor.as_ref().and_then(|a| a.line_number);
                let side = feedback.anchor.as_ref().and_then(|a| a.side);

                state.ui.active_feedback = Some(crate::ui::app::FeedbackContext {
                    feedback_id: Some(feedback.id),
                    task_id,
                    file_path,
                    line_number,
                    side,
                });
            } else {
                state.ui.selected_task_id = None;
                state.ui.active_feedback = None;
            }

            // 3. Ensure we are on the Review view
            state.ui.current_view = AppView::Review;

            Vec::new()
        }
        ReviewAction::RefreshFromDb { reason } => vec![Command::RefreshReviewData { reason }],
        ReviewAction::RefreshGitHubReview => {
            let Some(review_id) = state.ui.selected_review_id.clone() else {
                return Vec::new();
            };
            let Some(review) = state.domain.reviews.iter().find(|r| r.id == review_id) else {
                return Vec::new();
            };
            if !matches!(review.source, crate::domain::ReviewSource::GitHubPr { .. }) {
                state.ui.review_error = Some("Selected review is not a GitHub PR".into());
                return Vec::new();
            }

            state.ui.review_error = None;
            state.session.generation_error = None;
            state.session.is_generating = true;
            state.reset_agent_timeline();
            state.session.generate_preview = None;
            state.ui.current_view = AppView::Generate;

            vec![Command::RefreshGitHubReview {
                review_id,
                selected_agent_id: state.session.selected_agent.id.clone(),
            }]
        }
        ReviewAction::SelectReview { review_id } => {
            state.ui.selected_review_id = Some(review_id.clone());
            state.ui.selected_run_id = state
                .domain
                .reviews
                .iter()
                .find(|r| r.id == review_id)
                .and_then(|r| r.active_run_id.clone());

            state.ui.selected_task_id = None;
            state.ui.active_feedback = None;
            state.ui.show_push_feedback_modal = None;
            state.ui.push_feedback_error = None;
            state.ui.push_feedback_pending = None;

            let mut commands = select_default_task_for_current_run(state);
            commands.push(Command::LoadReviewFeedbacks {
                review_id: review_id.clone(),
            });
            commands.push(Command::LoadFeedbackLinks { review_id });
            commands
        }
        ReviewAction::SelectRun { run_id } => {
            state.ui.selected_run_id = Some(run_id);
            state.ui.selected_task_id = None;
            state.ui.active_feedback = None;
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
            state.ui.selected_task_id = None;
            state.ui.active_feedback = None;
            Vec::new()
        }
        ReviewAction::UpdateTaskStatus { task_id, status } => {
            state.ui.review_error = None;
            vec![Command::UpdateTaskStatus { task_id, status }]
        }
        ReviewAction::DeleteReview(review_id) => {
            state.ui.review_error = None;
            // If the deleted review was selected, deselect it
            if state.ui.selected_review_id.as_ref() == Some(&review_id) {
                state.ui.selected_review_id = None;
                state.ui.selected_run_id = None;
                state.ui.selected_task_id = None;
                state.ui.active_feedback = None;
            }
            vec![Command::DeleteReview { review_id }]
        }
        ReviewAction::CreateFeedbackComment {
            task_id,
            feedback_id,
            file_path,
            line_number,
            side,
            title,
            body,
        } => {
            let review_id = match state.ui.selected_review_id.clone() {
                Some(id) => id,
                None => return Vec::new(),
            };
            state.ui.review_error = None;
            vec![Command::CreateFeedbackComment {
                review_id,
                task_id,
                feedback_id,
                file_path,
                line_number,
                side,
                title,
                body,
            }]
        }
        ReviewAction::UpdateFeedbackStatus {
            feedback_id,
            status,
        } => {
            state.ui.review_error = None;
            let mut commands = vec![Command::UpdateFeedbackStatus {
                feedback_id: feedback_id.clone(),
                status,
            }];
            if !update_feedback_in_state(state, &feedback_id, |feedback| {
                feedback.status = status;
                feedback.updated_at = Utc::now().to_rfc3339();
            }) && let Some(review_id) = state.ui.selected_review_id.as_ref()
            {
                commands.push(Command::LoadReviewFeedbacks {
                    review_id: review_id.clone(),
                });
            }
            commands
        }
        ReviewAction::UpdateFeedbackImpact {
            feedback_id,
            impact,
        } => {
            state.ui.review_error = None;
            let mut commands = vec![Command::UpdateFeedbackImpact {
                feedback_id: feedback_id.clone(),
                impact,
            }];
            if !update_feedback_in_state(state, &feedback_id, |feedback| {
                feedback.impact = impact;
                feedback.updated_at = Utc::now().to_rfc3339();
            }) && let Some(review_id) = state.ui.selected_review_id.as_ref()
            {
                commands.push(Command::LoadReviewFeedbacks {
                    review_id: review_id.clone(),
                });
            }
            commands
        }
        ReviewAction::UpdateFeedbackTitle { feedback_id, title } => {
            state.ui.review_error = None;
            let mut commands = vec![Command::UpdateFeedbackTitle {
                feedback_id: feedback_id.clone(),
                title: title.clone(),
            }];
            if !update_feedback_in_state(state, &feedback_id, |feedback| {
                feedback.title = title.clone();
                feedback.updated_at = Utc::now().to_rfc3339();
            }) && let Some(review_id) = state.ui.selected_review_id.as_ref()
            {
                commands.push(Command::LoadReviewFeedbacks {
                    review_id: review_id.clone(),
                });
            }
            commands
        }
        ReviewAction::OpenFeedback {
            task_id,
            feedback_id,
            file_path,
            line_number,
            side,
        } => {
            // Initialize title draft from existing thread data
            state.ui.active_feedback = Some(crate::ui::app::FeedbackContext {
                feedback_id,
                task_id,
                file_path,
                line_number,
                side,
            });
            Vec::new()
        }
        ReviewAction::CloseFeedback => {
            state.ui.active_feedback = None;
            Vec::new()
        }
        ReviewAction::OpenFullDiff(view) => {
            state.ui.full_diff = Some(view);
            Vec::new()
        }
        ReviewAction::CloseFullDiff => {
            state.ui.full_diff = None;
            Vec::new()
        }
        ReviewAction::RequestExportPreview => {
            if let Some(review_id) = state.ui.selected_review_id.clone()
                && let Some(run_id) = state.ui.selected_run_id.clone()
            {
                state.ui.is_exporting = true;
                state.ui.review_error = None;
                // Default to selecting all feedbacks if none are selected yet
                let include_feedback_ids =
                    if state.ui.export_options.selected_feedback_ids.is_empty() {
                        None
                    } else {
                        Some(
                            state
                                .ui
                                .export_options
                                .selected_feedback_ids
                                .iter()
                                .cloned()
                                .collect(),
                        )
                    };

                vec![Command::GenerateExportPreview {
                    review_id,
                    run_id,
                    include_feedback_ids,
                    options: Box::new(map_export_options(&state.ui.export_options)),
                }]
            } else {
                state.ui.review_error = Some("No review or run selected for export".into());
                vec![]
            }
        }
        ReviewAction::CloseExportPreview => {
            state.ui.export_preview = None;
            Vec::new()
        }
        ReviewAction::ResetExportCopySuccess => {
            state.ui.export_copy_success = false;
            state.ui.export_copy_shown_frames = 0;
            Vec::new()
        }
        ReviewAction::ResetExportSaveSuccess => {
            state.ui.export_save_success = false;
            state.ui.export_save_shown_frames = 0;
            Vec::new()
        }
        ReviewAction::ExportReviewToFile { path } => {
            if let (Some(review_id), Some(run_id)) = (
                state.ui.selected_review_id.as_ref(),
                state.ui.selected_run_id.as_ref(),
            ) {
                state.ui.is_exporting = true;
                vec![Command::ExportReview {
                    review_id: review_id.clone(),
                    run_id: run_id.clone(),
                    path,
                    options: Box::new(map_export_options(&state.ui.export_options)),
                }]
            } else {
                Vec::new()
            }
        }
        ReviewAction::OpenInEditor {
            file_path,
            line_number,
        } => {
            state.ui.review_error = None;

            let Some(request) = resolve_editor_open_request(state, &file_path, line_number) else {
                state.ui.review_error =
                    Some("Link a repository to open files in an editor.".to_string());
                return Vec::new();
            };

            if let Some(editor_id) = state.ui.preferred_editor_id.clone() {
                if crate::infra::editor::is_editor_available(&editor_id) {
                    state.ui.pending_editor_open = None;
                    state.ui.show_editor_picker = false;
                    state.ui.editor_picker_error = None;
                    return vec![Command::OpenInEditor {
                        editor_id,
                        file_path: request.file_path,
                        line_number: request.line_number,
                    }];
                }

                state.ui.editor_picker_error =
                    Some("Preferred editor is not available on this system.".to_string());
            } else {
                state.ui.editor_picker_error = None;
            }

            state.ui.pending_editor_open = Some(request);
            state.ui.show_editor_picker = true;
            Vec::new()
        }
        ReviewAction::ToggleExportOptionsMenu => {
            state.ui.show_export_options_menu = !state.ui.show_export_options_menu;
            Vec::new()
        }
        ReviewAction::SelectAllExportFeedbacks => {
            state.ui.export_options.selected_feedback_ids = state
                .domain
                .feedbacks
                .iter()
                .map(|t| t.id.clone())
                .collect();
            let (review_id, run_id) = if let Some(r) = state.domain.reviews.first() {
                (r.id.clone(), r.active_run_id.clone())
            } else {
                return Vec::new();
            };
            let Some(run_id) = run_id else {
                return Vec::new();
            };
            vec![Command::GenerateExportPreview {
                review_id,
                run_id,
                include_feedback_ids: Some(
                    state
                        .ui
                        .export_options
                        .selected_feedback_ids
                        .iter()
                        .cloned()
                        .collect(),
                ),
                options: Box::new(map_export_options(&state.ui.export_options)),
            }]
        }
        ReviewAction::ClearExportFeedbacks => {
            state.ui.export_options.selected_feedback_ids.clear();
            let (review_id, run_id) = if let Some(r) = state.domain.reviews.first() {
                (r.id.clone(), r.active_run_id.clone())
            } else {
                return Vec::new();
            };
            let Some(run_id) = run_id else {
                return Vec::new();
            };
            vec![Command::GenerateExportPreview {
                review_id,
                run_id,
                include_feedback_ids: Some(Vec::new()),
                options: Box::new(map_export_options(&state.ui.export_options)),
            }]
        }
        ReviewAction::ToggleFeedbackSelection(feedback_id) => {
            if state
                .ui
                .export_options
                .selected_feedback_ids
                .contains(&feedback_id)
            {
                state
                    .ui
                    .export_options
                    .selected_feedback_ids
                    .remove(&feedback_id);
            } else {
                state
                    .ui
                    .export_options
                    .selected_feedback_ids
                    .insert(feedback_id);
            }
            // Trigger regeneration
            if let Some(review_id) = state.ui.selected_review_id.clone()
                && let Some(run_id) = state.ui.selected_run_id.clone()
            {
                state.ui.is_exporting = true;
                let include_feedback_ids =
                    if state.ui.export_options.selected_feedback_ids.is_empty() {
                        None
                    } else {
                        Some(
                            state
                                .ui
                                .export_options
                                .selected_feedback_ids
                                .iter()
                                .cloned()
                                .collect(), // This already converts HashSet to Vec
                        )
                    };

                vec![Command::GenerateExportPreview {
                    review_id,
                    run_id,
                    include_feedback_ids,
                    options: Box::new(map_export_options(&state.ui.export_options)),
                }]
            } else {
                Vec::new()
            }
        }
        ReviewAction::UpdateExportOptions(options) => {
            state.ui.export_options = options.clone();
            // Trigger regeneration
            if let Some(review_id) = state.ui.selected_review_id.clone()
                && let Some(run_id) = state.ui.selected_run_id.clone()
            {
                state.ui.is_exporting = true;
                let include_feedback_ids =
                    if state.ui.export_options.selected_feedback_ids.is_empty() {
                        None
                    } else {
                        Some(
                            state
                                .ui
                                .export_options
                                .selected_feedback_ids
                                .iter()
                                .cloned()
                                .collect(),
                        ) // This already converts HashSet to Vec
                    };

                vec![Command::GenerateExportPreview {
                    review_id,
                    run_id,
                    include_feedback_ids,
                    options: Box::new(map_export_options(&state.ui.export_options)),
                }]
            } else {
                Vec::new()
            }
        }
        ReviewAction::DeleteFeedback(feedback_id) => {
            if state
                .ui
                .active_feedback
                .as_ref()
                .map(|f| matches!(f.feedback_id.as_ref(), Some(id) if id == &feedback_id))
                .unwrap_or(false)
            {
                state.ui.active_feedback = None;
            }
            vec![Command::DeleteFeedback(feedback_id)]
        }
        ReviewAction::DeleteComment {
            feedback_id: _,
            comment_id,
        } => {
            vec![Command::DeleteComment(comment_id)]
        }
        ReviewAction::ShowSendFeedbackConfirm { feedback_id } => {
            state.ui.push_feedback_error = None;
            state.ui.show_push_feedback_modal = Some(feedback_id);
            Vec::new()
        }
        ReviewAction::CancelSendFeedbackConfirm => {
            state.ui.show_push_feedback_modal = None;
            state.ui.push_feedback_error = None;
            Vec::new()
        }
        ReviewAction::SendFeedbackToPr { feedback_id } => {
            state.ui.push_feedback_pending = Some(feedback_id.clone());
            state.ui.push_feedback_error = None;
            vec![Command::SendFeedbackToPr { feedback_id }]
        }
        ReviewAction::OpenSendToPrModal => {
            state.ui.send_to_pr_modal_open = true;
            state.ui.send_to_pr_pending = false;
            state.ui.send_to_pr_error = None;
            state.ui.send_to_pr_include_summary = true;
            state.ui.send_to_pr_selection = default_send_to_pr_selection(state);
            Vec::new()
        }
        ReviewAction::CloseSendToPrModal => {
            state.ui.send_to_pr_modal_open = false;
            state.ui.send_to_pr_pending = false;
            state.ui.send_to_pr_error = None;
            Vec::new()
        }
        ReviewAction::ToggleSendToPrSummary { include } => {
            state.ui.send_to_pr_include_summary = include;
            Vec::new()
        }
        ReviewAction::ToggleSendToPrFeedback { feedback_id } => {
            if state.ui.send_to_pr_selection.contains(&feedback_id) {
                state.ui.send_to_pr_selection.remove(&feedback_id);
            } else {
                state.ui.send_to_pr_selection.insert(feedback_id);
            }
            Vec::new()
        }
        ReviewAction::ConfirmSendToPr => {
            let Some(review_id) = state.ui.selected_review_id.clone() else {
                state.ui.send_to_pr_error = Some("Select a review first.".into());
                return Vec::new();
            };
            state.ui.send_to_pr_pending = true;
            state.ui.send_to_pr_error = None;
            let feedback_ids: Vec<String> = state.ui.send_to_pr_selection.iter().cloned().collect();
            vec![Command::SendFeedbacksToPr {
                review_id,
                feedback_ids,
                include_summary: state.ui.send_to_pr_include_summary,
            }]
        }
    }
}

pub fn select_task(state: &mut AppState, task_id: TaskId) -> Vec<Command> {
    state.ui.selected_task_id = Some(task_id.clone());
    state.ui.active_feedback = None;
    let _ = task_id;
    Vec::new()
}

pub fn select_default_task_for_current_run(state: &mut AppState) -> Vec<Command> {
    let current_tasks = state.tasks();
    let Some(next_open) = current_tasks
        .iter()
        .find(|t| matches!(t.status, ReviewStatus::Todo | ReviewStatus::InProgress))
    else {
        return Vec::new();
    };

    state.ui.selected_task_id = Some(next_open.id.clone());
    Vec::new()
}

pub fn update_feedback_in_state<F>(state: &mut AppState, feedback_id: &str, mut updater: F) -> bool
where
    F: FnMut(&mut crate::domain::Feedback),
{
    if let Some(feedback) = state
        .domain
        .feedbacks
        .iter_mut()
        .find(|t| t.id == feedback_id)
    {
        updater(feedback);
        true
    } else {
        false
    }
}

fn default_send_to_pr_selection(state: &AppState) -> HashSet<String> {
    let mut selection = HashSet::new();
    let Some(review_id) = state.ui.selected_review_id.as_ref() else {
        return selection;
    };

    let Some(review) = state.domain.reviews.iter().find(|r| &r.id == review_id) else {
        return selection;
    };

    if !matches!(review.source, crate::domain::ReviewSource::GitHubPr { .. }) {
        return selection;
    }

    for feedback in state
        .domain
        .feedbacks
        .iter()
        .filter(|f| &f.review_id == review_id)
    {
        if let Some(anchor) = &feedback.anchor {
            if anchor.file_path.is_some() && anchor.line_number.is_some() && anchor.side.is_some() {
                selection.insert(feedback.id.clone());
            }
        }
    }

    selection
}

pub fn apply_review_data(state: &mut AppState, payload: ReviewDataPayload) -> Vec<Command> {
    state.domain.reviews = payload.reviews;
    state.domain.runs = payload.runs;
    state.domain.all_tasks = payload.tasks;
    state.ui.review_error = None;

    if let Some(selected) = &state.ui.selected_review_id
        && !state.domain.reviews.iter().any(|r| &r.id == selected)
    {
        state.ui.selected_review_id = None;
    }

    if state.ui.selected_review_id.is_none() {
        state.ui.selected_review_id = state.domain.reviews.first().map(|r| r.id.clone());
    }

    if let Some(selected_review_id) = &state.ui.selected_review_id {
        let default_run_id = state
            .domain
            .reviews
            .iter()
            .find(|r| &r.id == selected_review_id)
            .and_then(|r| r.active_run_id.clone());

        let run_in_review = state.ui.selected_run_id.as_ref().is_some_and(|run_id| {
            state
                .domain
                .runs
                .iter()
                .any(|run| &run.id == run_id && &run.review_id == selected_review_id)
        });

        if !run_in_review {
            state.ui.selected_run_id = default_run_id;
        }

        let current_tasks = state.tasks();

        if let Some(selected_task_id) = &state.ui.selected_task_id
            && !current_tasks.iter().any(|t| &t.id == selected_task_id)
        {
            state.ui.selected_task_id = None;
        }

        let mut commands = Vec::new();

        if let Some(review_id) = state.ui.selected_review_id.clone() {
            commands.push(Command::LoadReviewFeedbacks {
                review_id: review_id.clone(),
            });
            commands.push(Command::LoadFeedbackLinks { review_id });
        }

        if state.ui.selected_task_id.is_none()
            && let Some(next_open) = current_tasks
                .iter()
                .find(|t| matches!(t.status, ReviewStatus::Todo | ReviewStatus::InProgress))
        {
            state.ui.selected_task_id = Some(next_open.id.clone());
        }

        commands
    } else {
        Vec::new()
    }
}

fn resolve_editor_open_request(
    state: &AppState,
    file_path: &str,
    line_number: usize,
) -> Option<crate::ui::app::state::EditorOpenRequest> {
    if state.domain.linked_repos.is_empty() {
        return None;
    }

    let rel_path = Path::new(file_path);
    if rel_path.is_absolute() {
        return None;
    }

    let selected_repo = state.ui.selected_repo_id.as_ref().and_then(|selected_id| {
        state
            .domain
            .linked_repos
            .iter()
            .find(|repo| &repo.id == selected_id)
    });

    let mut repos = Vec::new();
    if let Some(selected) = selected_repo {
        repos.push(selected);
    }

    for repo in &state.domain.linked_repos {
        if !repos.iter().any(|existing| existing.id == repo.id) {
            repos.push(repo);
        }
    }

    for repo in &repos {
        let candidate = repo.path.join(rel_path);
        if candidate.exists() {
            return Some(crate::ui::app::state::EditorOpenRequest {
                file_path: candidate,
                line_number,
            });
        }
    }

    let fallback_repo = if let Some(selected) = selected_repo {
        Some(selected)
    } else if state.domain.linked_repos.len() == 1 {
        state.domain.linked_repos.first()
    } else {
        None
    };

    fallback_repo.map(|repo| crate::ui::app::state::EditorOpenRequest {
        file_path: repo.path.join(rel_path),
        line_number,
    })
}

fn map_export_options(
    ui_options: &crate::ui::app::state::ExportOptions,
) -> crate::application::review::export::ExportOptions {
    crate::application::review::export::ExportOptions {
        include_summary: ui_options.include_summary,
        include_stats: ui_options.include_stats,
        include_metadata: ui_options.include_metadata,
        include_tasks: ui_options.include_tasks,
        include_feedbacks: ui_options.include_feedbacks,
        include_feedback_ids: if ui_options.selected_feedback_ids.is_empty() {
            None
        } else {
            Some(ui_options.selected_feedback_ids.clone())
        },
    }
}
