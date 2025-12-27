use super::super::super::state::{AppState, AppView};
use super::super::action::{ReviewAction, ReviewDataPayload};
use super::super::command::Command;
use crate::domain::{ReviewStatus, TaskId};
use chrono::Utc;
use std::path::Path;

pub fn reduce(state: &mut AppState, action: ReviewAction) -> Vec<Command> {
    match action {
        ReviewAction::NavigateToThread(thread) => {
            // 1. Select the task (this usually clears active_thread, so we do it first)
            if let Some(task_id) = thread.task_id.clone() {
                select_task(state, task_id.clone());

                // 2. Set the active thread context
                let file_path = thread.anchor.as_ref().and_then(|a| a.file_path.clone());
                let line_number = thread.anchor.as_ref().and_then(|a| a.line_number);

                state.ui.thread_title_draft = thread.title.clone();
                state.ui.thread_reply_draft.clear();

                state.ui.active_thread = Some(crate::ui::app::ThreadContext {
                    thread_id: Some(thread.id),
                    task_id,
                    file_path,
                    line_number,
                });
            } else {
                state.ui.selected_task_id = None;
                state.ui.active_thread = None;
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
            state.ui.cached_unified_diff = None;
            state.ui.active_thread = None;
            state.ui.thread_title_draft.clear();
            state.ui.thread_reply_draft.clear();

            let mut commands = select_default_task_for_current_run(state);
            commands.push(Command::LoadReviewThreads { review_id });
            commands
        }
        ReviewAction::SelectRun { run_id } => {
            state.ui.selected_run_id = Some(run_id);
            state.ui.selected_task_id = None;
            state.ui.cached_unified_diff = None;
            state.ui.active_thread = None;
            state.ui.thread_title_draft.clear();
            state.ui.thread_reply_draft.clear();
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
            state.ui.cached_unified_diff = None;
            state.ui.active_thread = None;
            state.ui.thread_title_draft.clear();
            state.ui.thread_reply_draft.clear();
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
                state.ui.active_thread = None;
            }
            vec![Command::DeleteReview { review_id }]
        }
        ReviewAction::CreateThreadComment {
            task_id,
            thread_id,
            file_path,
            line_number,
            title,
            body,
        } => {
            let review_id = match state.ui.selected_review_id.clone() {
                Some(id) => id,
                None => return Vec::new(),
            };
            state.ui.review_error = None;
            vec![Command::CreateThreadComment {
                review_id,
                task_id,
                thread_id,
                file_path,
                line_number,
                title,
                body,
            }]
        }
        ReviewAction::UpdateThreadStatus { thread_id, status } => {
            state.ui.review_error = None;
            update_thread_in_state(state, &thread_id, |thread| {
                thread.status = status;
                thread.updated_at = Utc::now().to_rfc3339();
            });
            vec![Command::UpdateThreadStatus { thread_id, status }]
        }
        ReviewAction::UpdateThreadImpact { thread_id, impact } => {
            state.ui.review_error = None;
            update_thread_in_state(state, &thread_id, |thread| {
                thread.impact = impact;
                thread.updated_at = Utc::now().to_rfc3339();
            });
            vec![Command::UpdateThreadImpact { thread_id, impact }]
        }
        ReviewAction::UpdateThreadTitle { thread_id, title } => {
            state.ui.review_error = None;
            update_thread_in_state(state, &thread_id, |thread| {
                thread.title = title.clone();
                thread.updated_at = Utc::now().to_rfc3339();
            });
            vec![Command::UpdateThreadTitle { thread_id, title }]
        }
        ReviewAction::OpenThread {
            task_id,
            thread_id,
            file_path,
            line_number,
        } => {
            // Initialize title draft from existing thread data
            let existing_title = thread_id
                .as_ref()
                .and_then(|tid| state.domain.threads.iter().find(|t| &t.id == tid))
                .map(|t| t.title.clone())
                .unwrap_or_default();
            state.ui.thread_title_draft = existing_title;
            state.ui.thread_reply_draft.clear();

            state.ui.active_thread = Some(crate::ui::app::ThreadContext {
                thread_id,
                task_id,
                file_path,
                line_number,
            });
            Vec::new()
        }
        ReviewAction::CloseThread => {
            state.ui.thread_title_draft.clear();
            state.ui.thread_reply_draft.clear();
            state.ui.active_thread = None;
            Vec::new()
        }
        ReviewAction::SetThreadTitleDraft { text } => {
            state.ui.thread_title_draft = text;
            Vec::new()
        }
        ReviewAction::SetThreadReplyDraft { text } => {
            state.ui.thread_reply_draft = text;
            Vec::new()
        }
        ReviewAction::ClearThreadReplyDraft => {
            state.ui.thread_reply_draft.clear();
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
                vec![Command::GenerateExportPreview { review_id, run_id }]
            } else {
                state.ui.review_error = Some("No review or run selected for export".into());
                vec![]
            }
        }
        ReviewAction::CloseExportPreview => {
            state.ui.export_preview = None;
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
    }
}

pub fn select_task(state: &mut AppState, task_id: TaskId) -> Vec<Command> {
    state.ui.selected_task_id = Some(task_id.clone());
    state.ui.cached_unified_diff = None;
    state.ui.active_thread = None;
    state.ui.thread_title_draft.clear();
    state.ui.thread_reply_draft.clear();
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

pub fn update_thread_in_state<F>(state: &mut AppState, thread_id: &str, mut updater: F)
where
    F: FnMut(&mut crate::domain::Thread),
{
    if let Some(thread) = state.domain.threads.iter_mut().find(|t| t.id == thread_id) {
        updater(thread);
    }
}

pub fn apply_review_data(state: &mut AppState, payload: ReviewDataPayload) -> Vec<Command> {
    state.domain.reviews = payload.reviews;
    state.domain.runs = payload.runs;
    state.domain.all_tasks = payload.tasks;
    state.ui.review_error = None;
    state.ui.cached_unified_diff = None;

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
            commands.push(Command::LoadReviewThreads { review_id });
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
