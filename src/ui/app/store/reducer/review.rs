use super::super::super::state::{AppState, AppView};
use super::super::action::{ReviewAction, ReviewDataPayload};
use super::super::command::Command;
use crate::domain::{TaskId, TaskStatus};
use chrono::Utc;

pub fn reduce(state: &mut AppState, action: ReviewAction) -> Vec<Command> {
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
            state.cached_unified_diff = None;
            state.active_thread = None;
            state.thread_title_draft.clear();
            state.thread_reply_draft.clear();

            let mut commands = select_default_task_for_current_run(state);
            commands.push(Command::LoadReviewThreads { review_id });
            commands
        }
        ReviewAction::SelectRun { run_id } => {
            state.selected_run_id = Some(run_id);
            state.selected_task_id = None;
            state.cached_unified_diff = None;
            state.active_thread = None;
            state.thread_title_draft.clear();
            state.thread_reply_draft.clear();
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
            state.cached_unified_diff = None;
            state.active_thread = None;
            state.thread_title_draft.clear();
            state.thread_reply_draft.clear();
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
        ReviewAction::CreateThreadComment {
            task_id,
            thread_id,
            file_path,
            line_number,
            title,
            body,
        } => {
            let review_id = match state.selected_review_id.clone() {
                Some(id) => id,
                None => return Vec::new(),
            };
            state.review_error = None;
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
            state.review_error = None;
            update_thread_in_state(state, &thread_id, |thread| {
                thread.status = status;
                thread.updated_at = Utc::now().to_rfc3339();
            });
            vec![Command::UpdateThreadStatus { thread_id, status }]
        }
        ReviewAction::UpdateThreadImpact { thread_id, impact } => {
            state.review_error = None;
            update_thread_in_state(state, &thread_id, |thread| {
                thread.impact = impact;
                thread.updated_at = Utc::now().to_rfc3339();
            });
            vec![Command::UpdateThreadImpact { thread_id, impact }]
        }
        ReviewAction::UpdateThreadTitle { thread_id, title } => {
            state.review_error = None;
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
                .and_then(|tid| state.threads.iter().find(|t| &t.id == tid))
                .map(|t| t.title.clone())
                .unwrap_or_default();
            state.thread_title_draft = existing_title;
            state.thread_reply_draft.clear();

            state.active_thread = Some(crate::ui::app::ThreadContext {
                thread_id,
                task_id,
                file_path,
                line_number,
            });
            Vec::new()
        }
        ReviewAction::CloseThread => {
            state.thread_title_draft.clear();
            state.thread_reply_draft.clear();
            state.active_thread = None;
            Vec::new()
        }
        ReviewAction::SetThreadTitleDraft { text } => {
            state.thread_title_draft = text;
            Vec::new()
        }
        ReviewAction::SetThreadReplyDraft { text } => {
            state.thread_reply_draft = text;
            Vec::new()
        }
        ReviewAction::ClearThreadReplyDraft => {
            state.thread_reply_draft.clear();
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
            if let (Some(review_id), Some(run_id)) = (
                state.selected_review_id.as_ref(),
                state.selected_run_id.as_ref(),
            ) {
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
    }
}

pub fn select_task(state: &mut AppState, task_id: TaskId) -> Vec<Command> {
    state.selected_task_id = Some(task_id.clone());
    state.cached_unified_diff = None;
    state.active_thread = None;
    state.thread_title_draft.clear();
    state.thread_reply_draft.clear();
    let _ = task_id;
    Vec::new()
}

pub fn select_default_task_for_current_run(state: &mut AppState) -> Vec<Command> {
    let current_tasks = state.tasks();
    let Some(next_open) = current_tasks
        .iter()
        .find(|t| matches!(t.status, TaskStatus::Pending | TaskStatus::InProgress))
    else {
        return Vec::new();
    };

    state.selected_task_id = Some(next_open.id.clone());
    Vec::new()
}

pub fn update_thread_in_state<F>(state: &mut AppState, thread_id: &str, mut updater: F)
where
    F: FnMut(&mut crate::domain::Thread),
{
    if let Some(thread) = state.threads.iter_mut().find(|t| t.id == thread_id) {
        updater(thread);
    }
}

pub fn apply_review_data(state: &mut AppState, payload: ReviewDataPayload) -> Vec<Command> {
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

        let current_tasks = state.tasks();

        if let Some(selected_task_id) = &state.selected_task_id
            && !current_tasks.iter().any(|t| &t.id == selected_task_id)
        {
            state.selected_task_id = None;
        }

        let mut commands = Vec::new();

        if let Some(review_id) = state.selected_review_id.clone() {
            commands.push(Command::LoadReviewThreads { review_id });
        }

        if state.selected_task_id.is_none()
            && let Some(next_open) = current_tasks
                .iter()
                .find(|t| matches!(t.status, TaskStatus::Pending | TaskStatus::InProgress))
        {
            state.selected_task_id = Some(next_open.id.clone());
        }

        commands
    } else {
        Vec::new()
    }
}
