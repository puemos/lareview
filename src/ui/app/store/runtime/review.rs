use super::super::super::LaReviewApp;
use super::super::action::{
    Action, AsyncAction, ReviewAction, ReviewDataPayload, ReviewThreadsPayload,
};
use super::super::command::ReviewDataRefreshReason;
use crate::domain::{Comment, ReviewId, ReviewStatus, Thread, ThreadAnchor, ThreadImpact};
use std::collections::HashMap;

pub fn refresh_review_data(app: &mut LaReviewApp, reason: ReviewDataRefreshReason) {
    let result = (|| -> Result<ReviewDataPayload, String> {
        let reviews = app
            .review_repo
            .list_all()
            .map_err(|e| format!("Failed to load reviews: {e}"))?;
        let runs = app
            .run_repo
            .list_all()
            .map_err(|e| format!("Failed to load review runs: {e}"))?;
        let tasks = app
            .task_repo
            .find_all()
            .map_err(|e| format!("Failed to load tasks: {e}"))?;
        Ok(ReviewDataPayload {
            reviews,
            runs,
            tasks,
        })
    })();

    app.dispatch(Action::Async(AsyncAction::ReviewDataLoaded {
        reason,
        result,
    }));
}

pub fn load_review_threads(app: &mut LaReviewApp, review_id: ReviewId) {
    let result = (|| -> Result<ReviewThreadsPayload, String> {
        let threads = app
            .thread_repo
            .find_by_review(&review_id)
            .map_err(|e| format!("Failed to load threads: {e}"))?;
        let mut comments = HashMap::new();
        for thread in &threads {
            let thread_comments = app
                .comment_repo
                .list_for_thread(&thread.id)
                .map_err(|e| format!("Failed to load comments: {e}"))?;
            comments.insert(thread.id.clone(), thread_comments);
        }
        Ok(ReviewThreadsPayload {
            review_id,
            threads,
            comments,
        })
    })();

    app.dispatch(Action::Async(AsyncAction::ReviewThreadsLoaded(result)));
}

pub fn update_task_status(
    app: &mut LaReviewApp,
    task_id: crate::domain::TaskId,
    status: crate::domain::ReviewStatus,
) {
    let result = app
        .task_repo
        .update_status(&task_id, status)
        .map_err(|e| format!("Failed to update task status: {e}"));

    app.dispatch(Action::Async(AsyncAction::TaskStatusSaved(result)));
}

pub fn delete_review(app: &mut LaReviewApp, review_id: ReviewId) {
    let result = (|| -> Result<(), String> {
        let runs = app
            .run_repo
            .find_by_review_id(&review_id)
            .map_err(|e| format!("Failed to fetch runs for review: {e}"))?;

        if !runs.is_empty() {
            let run_ids: Vec<_> = runs.iter().map(|r| r.id.clone()).collect();
            let tasks = app
                .task_repo
                .find_by_run_ids(&run_ids)
                .map_err(|e| format!("Failed to fetch tasks for runs: {e}"))?;

            if !tasks.is_empty() {
                let task_ids: Vec<_> = tasks.iter().map(|t| t.id.clone()).collect();

                app.task_repo
                    .delete_by_ids(&task_ids)
                    .map_err(|e| format!("Failed to delete tasks: {e}"))?;
            }

            app.run_repo
                .delete_by_review_id(&review_id)
                .map_err(|e| format!("Failed to delete runs: {e}"))?;
        }

        app.review_repo
            .delete(&review_id)
            .map_err(|e| format!("Failed to delete review: {e}"))?;

        Ok(())
    })();
    app.dispatch(Action::Async(AsyncAction::ReviewDeleted(result)));
}

#[allow(clippy::too_many_arguments)]
pub fn create_thread_comment(
    app: &mut LaReviewApp,
    review_id: ReviewId,
    task_id: crate::domain::TaskId,
    thread_id: Option<String>,
    file_path: Option<String>,
    line_number: Option<u32>,
    title: Option<String>,
    body: String,
) {
    let now = chrono::Utc::now().to_rfc3339();
    let is_new_thread = thread_id.is_none();
    let thread_id = thread_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let result = (|| -> Result<(), String> {
        if is_new_thread {
            let title = title.unwrap_or_else(|| default_thread_title(&body));
            let anchor = if file_path.is_some() || line_number.is_some() {
                Some(ThreadAnchor {
                    file_path: file_path.clone(),
                    line_number,
                    side: None,
                    hunk_ref: None,
                    head_sha: None,
                })
            } else {
                None
            };

            let thread = Thread {
                id: thread_id.clone(),
                review_id: review_id.clone(),
                task_id: Some(task_id.clone()),
                title,
                status: ReviewStatus::Todo,
                impact: ThreadImpact::Nitpick,
                anchor,
                author: "User".to_string(),
                created_at: now.clone(),
                updated_at: now.clone(),
            };

            app.thread_repo
                .save(&thread)
                .map_err(|e| format!("Failed to save thread: {e}"))?;
        } else {
            app.thread_repo
                .touch(&thread_id)
                .map_err(|e| format!("Failed to update thread timestamp: {e}"))?;
        }

        let comment = Comment {
            id: uuid::Uuid::new_v4().to_string(),
            thread_id: thread_id.clone(),
            author: "User".to_string(),
            body,
            parent_id: None,
            created_at: now.clone(),
            updated_at: now,
        };

        app.comment_repo
            .save(&comment)
            .map_err(|e| format!("Failed to save comment: {e}"))?;

        Ok(())
    })();

    app.dispatch(Action::Async(AsyncAction::ThreadCommentSaved(
        result.clone(),
    )));

    if result.is_ok() {
        load_review_threads(app, review_id.clone());
        app.dispatch(Action::Review(ReviewAction::OpenThread {
            task_id,
            thread_id: Some(thread_id),
            file_path,
            line_number,
        }));
    }
}

pub fn update_thread_status(app: &mut LaReviewApp, thread_id: String, status: ReviewStatus) {
    let review_id = app.state.ui.selected_review_id.clone();
    let result = app
        .thread_repo
        .update_status(&thread_id, status)
        .map(|_| ())
        .map_err(|e| format!("Failed to update thread status: {e}"));

    app.dispatch(Action::Async(AsyncAction::ThreadCommentSaved(
        result.clone(),
    )));

    if let (Ok(_), Some(review_id)) = (result, review_id) {
        load_review_threads(app, review_id);
    }
}

pub fn update_thread_impact(app: &mut LaReviewApp, thread_id: String, impact: ThreadImpact) {
    let review_id = app.state.ui.selected_review_id.clone();
    let result = app
        .thread_repo
        .update_impact(&thread_id, impact)
        .map(|_| ())
        .map_err(|e| format!("Failed to update thread impact: {e}"));

    app.dispatch(Action::Async(AsyncAction::ThreadCommentSaved(
        result.clone(),
    )));

    if let (Ok(_), Some(review_id)) = (result, review_id) {
        load_review_threads(app, review_id);
    }
}

pub fn update_thread_title(app: &mut LaReviewApp, thread_id: String, title: String) {
    let review_id = app.state.ui.selected_review_id.clone();
    let result = app
        .thread_repo
        .update_title(&thread_id, &title)
        .map(|_| ())
        .map_err(|e| format!("Failed to update thread title: {e}"));

    app.dispatch(Action::Async(AsyncAction::ThreadCommentSaved(
        result.clone(),
    )));

    if let (Ok(_), Some(review_id)) = (result, review_id) {
        load_review_threads(app, review_id);
    }
}

pub fn generate_export_preview(
    app: &mut LaReviewApp,
    review_id: crate::domain::ReviewId,
    run_id: crate::domain::ReviewRunId,
) {
    let review_repo = app.review_repo.clone();
    let run_repo = app.run_repo.clone();
    let task_repo = app.task_repo.clone();
    let thread_repo = app.thread_repo.clone();
    let comment_repo = app.comment_repo.clone();
    let action_tx = app.action_tx.clone();

    tokio::spawn(async move {
        let result = async {
            let review = review_repo
                .find_by_id(&review_id)
                .map_err(|e: anyhow::Error| e.to_string())?
                .ok_or("Review not found")?;
            let run = run_repo
                .find_by_id(&run_id)
                .map_err(|e: anyhow::Error| e.to_string())?
                .ok_or("Run not found")?;
            let tasks = task_repo
                .find_by_run(&run_id)
                .map_err(|e: anyhow::Error| e.to_string())?;

            let threads = thread_repo
                .find_by_review(&review_id)
                .map_err(|e: anyhow::Error| e.to_string())?;
            let mut comments = Vec::new();
            for thread in &threads {
                let mut thread_comments = comment_repo
                    .list_for_thread(&thread.id)
                    .map_err(|e: anyhow::Error| e.to_string())?;
                comments.append(&mut thread_comments);
            }

            let data = crate::application::review::export::ExportData {
                review,
                run,
                tasks,
                threads,
                comments,
            };

            crate::application::review::export::ReviewExporter::export_to_markdown(&data, true)
                .await
                .map_err(|e| e.to_string())
        }
        .await;

        let _ = action_tx
            .send(Action::Async(AsyncAction::ExportPreviewGenerated(result)))
            .await;
    });
}

pub fn export_review(
    app: &mut LaReviewApp,
    review_id: crate::domain::ReviewId,
    run_id: crate::domain::ReviewRunId,
    path: std::path::PathBuf,
) {
    let review_repo = app.review_repo.clone();
    let run_repo = app.run_repo.clone();
    let task_repo = app.task_repo.clone();
    let thread_repo = app.thread_repo.clone();
    let comment_repo = app.comment_repo.clone();
    let action_tx = app.action_tx.clone();

    tokio::spawn(async move {
        let result: anyhow::Result<()> = async {
            let review = review_repo
                .find_by_id(&review_id)
                .map_err(|e: anyhow::Error| anyhow::anyhow!(e))?
                .ok_or_else(|| anyhow::anyhow!("Review not found"))?;
            let run = run_repo
                .find_by_id(&run_id)
                .map_err(|e: anyhow::Error| anyhow::anyhow!(e))?
                .ok_or_else(|| anyhow::anyhow!("Run not found"))?;
            let tasks = task_repo
                .find_by_run(&run_id)
                .map_err(|e: anyhow::Error| anyhow::anyhow!(e))?;

            let threads = thread_repo
                .find_by_review(&review_id)
                .map_err(|e: anyhow::Error| anyhow::anyhow!(e))?;
            let mut comments = Vec::new();
            for thread in &threads {
                let mut thread_comments = comment_repo
                    .list_for_thread(&thread.id)
                    .map_err(|e: anyhow::Error| anyhow::anyhow!(e))?;
                comments.append(&mut thread_comments);
            }

            let data = crate::application::review::export::ExportData {
                review,
                run,
                tasks,
                threads,
                comments,
            };

            let export_result =
                crate::application::review::export::ReviewExporter::export_to_markdown(
                    &data, false,
                )
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            std::fs::write(&path, &export_result.markdown)?;

            if !export_result.assets.is_empty() {
                let parent_dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
                let assets_dir = parent_dir.join("assets");
                std::fs::create_dir_all(&assets_dir)?;

                for (filename, bytes) in export_result.assets {
                    std::fs::write(assets_dir.join(filename), bytes)?;
                }
            }

            Ok(())
        }
        .await;

        let _ = action_tx
            .send(Action::Async(AsyncAction::ExportFinished(
                result.map_err(|e: anyhow::Error| e.to_string()),
            )))
            .await;
    });
}

fn default_thread_title(body: &str) -> String {
    let first_line = body.lines().next().unwrap_or("").trim();
    if first_line.is_empty() {
        return "Untitled thread".to_string();
    }
    if first_line.len() > 80 {
        format!("{}...", &first_line[..77])
    } else {
        first_line.to_string()
    }
}
