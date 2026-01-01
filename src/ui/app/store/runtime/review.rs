use super::super::super::LaReviewApp;
use super::super::action::{
    Action, AsyncAction, ReviewAction, ReviewDataPayload, ReviewFeedbackLinksPayload,
    ReviewFeedbacksPayload,
};
use super::super::command::ReviewDataRefreshReason;
use crate::application::review::export::{ExportData, ExportOptions, ReviewExporter};
use crate::domain::{
    Comment, Feedback, FeedbackAnchor, FeedbackImpact, FeedbackSide, ReviewId, ReviewStatus,
};
use crate::ui::app::store::action::SendToPrResult;
use log::warn;
use std::collections::HashMap;
use unidiff::PatchSet;

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

pub fn load_review_feedbacks(app: &mut LaReviewApp, review_id: ReviewId) {
    let result = (|| -> Result<ReviewFeedbacksPayload, String> {
        let feedbacks = app
            .feedback_repo
            .find_by_review(&review_id)
            .map_err(|e| format!("Failed to load feedbacks: {e}"))?;
        let mut comments = HashMap::new();
        for feedback in &feedbacks {
            let feedback_comments = app
                .comment_repo
                .list_for_feedback(&feedback.id)
                .map_err(|e| format!("Failed to load comments: {e}"))?;
            comments.insert(feedback.id.clone(), feedback_comments);
        }
        Ok(ReviewFeedbacksPayload {
            review_id,
            feedbacks,
            comments,
        })
    })();

    app.dispatch(Action::Async(AsyncAction::ReviewFeedbacksLoaded(result)));
}

pub fn load_feedback_links(app: &mut LaReviewApp, review_id: ReviewId) {
    let result = (|| -> Result<ReviewFeedbackLinksPayload, String> {
        let feedbacks = app
            .feedback_repo
            .find_by_review(&review_id)
            .map_err(|e| format!("Failed to load feedbacks: {e}"))?;
        let ids: Vec<String> = feedbacks.iter().map(|f| f.id.clone()).collect();
        let repo = app.feedback_link_repo.clone();
        let mut links = std::collections::HashMap::new();
        for link in repo
            .find_by_feedback_ids(&ids)
            .map_err(|e| format!("Failed to load feedback links: {e}"))?
        {
            links.insert(link.feedback_id.clone(), link);
        }
        Ok(ReviewFeedbackLinksPayload { review_id, links })
    })();

    app.dispatch(Action::Async(AsyncAction::ReviewFeedbackLinksLoaded(
        result,
    )));
}

pub fn send_feedback_to_pr(app: &mut LaReviewApp, feedback_id: String) {
    let feedback_repo = app.feedback_repo.clone();
    let comment_repo = app.comment_repo.clone();
    let review_repo = app.review_repo.clone();
    let feedback_link_repo = app.feedback_link_repo.clone();
    let task_repo = app.task_repo.clone();
    let run_repo = app.run_repo.clone();
    let action_tx = app.action_tx.clone();

    tokio::spawn(async move {
        let result = async {
            let feedback = feedback_repo
                .find_by_id(&feedback_id)
                .map_err(|e| format!("Failed to load feedback: {e}"))?
                .ok_or_else(|| "Feedback not found".to_string())?;

            let review = review_repo
                .find_by_id(&feedback.review_id)
                .map_err(|e| format!("Failed to load review: {e}"))?
                .ok_or_else(|| "Review not found".to_string())?;

            let (owner, repo, number, head_sha) = match &review.source {
                crate::domain::ReviewSource::GitHubPr {
                    owner,
                    repo,
                    number,
                    head_sha,
                    ..
                } => (owner.clone(), repo.clone(), *number, head_sha.clone()),
                _ => return Err("Review is not a GitHub PR".to_string()),
            };

            let commit_id =
                head_sha.ok_or_else(|| "Missing PR head SHA; re-run PR fetch".to_string())?;
            let anchor = feedback
                .anchor
                .as_ref()
                .ok_or_else(|| "Feedback missing anchor; add it from the diff view.".to_string())?;
            let path = anchor
                .file_path
                .as_deref()
                .ok_or_else(|| "Feedback missing file path".to_string())?;
            let line = anchor
                .line_number
                .ok_or_else(|| "Feedback missing line number".to_string())?;
            let side = anchor.side.ok_or_else(|| {
                "Feedback missing side; add the feedback from the diff again.".to_string()
            })?;
            let task_id = feedback.task_id.clone().ok_or_else(|| {
                "Feedback is not linked to a task; open it from a task to send it to the PR."
                    .to_string()
            })?;

            let task = task_repo
                .find_by_id(&task_id)
                .map_err(|e| format!("Failed to load task: {e}"))?
                .ok_or_else(|| {
                    "Task not found for feedback; refresh the review data.".to_string()
                })?;

            let run = run_repo
                .find_by_id(&task.run_id)
                .map_err(|e| format!("Failed to load run: {e}"))?
                .ok_or_else(|| "Run not found for task; refresh the review data.".to_string())?;

            let position = compute_diff_position(&run.diff_text, path, line, side)?;

            let comments = comment_repo
                .list_for_feedback(&feedback.id)
                .map_err(|e| format!("Failed to load feedback comments: {e}"))?;

            let body = ReviewExporter::render_single_feedback_markdown(&feedback, &comments);

            let posted = crate::infra::vcs::github::create_review_comment(
                &owner, &repo, number, &body, &commit_id, path, position,
            )
            .await
            .map_err(|e| format!("Failed to post review comment: {e}"))?;

            let link = crate::domain::FeedbackLink {
                id: uuid::Uuid::new_v4().to_string(),
                feedback_id: feedback.id.clone(),
                provider: "github".to_string(),
                provider_feedback_id: posted.id.clone(),
                provider_root_comment_id: posted.url.clone().unwrap_or_else(|| posted.id.clone()),
                last_synced_at: chrono::Utc::now().to_rfc3339(),
            };

            feedback_link_repo
                .save(&link)
                .map_err(|e| format!("Failed to save feedback link: {e}"))?;

            Ok(link)
        }
        .await;

        let _ = action_tx
            .send(Action::Async(AsyncAction::FeedbackPushed(result)))
            .await;
    });
}

pub fn send_feedbacks_to_pr(
    app: &mut LaReviewApp,
    review_id: String,
    feedback_ids: Vec<String>,
    include_summary: bool,
) {
    let feedback_repo = app.feedback_repo.clone();
    let comment_repo = app.comment_repo.clone();
    let review_repo = app.review_repo.clone();
    let feedback_link_repo = app.feedback_link_repo.clone();
    let task_repo = app.task_repo.clone();
    let run_repo = app.run_repo.clone();
    let action_tx = app.action_tx.clone();

    tokio::spawn(async move {
        let result: Result<SendToPrResult, String> = async {
            let review = review_repo
                .find_by_id(&review_id)
                .map_err(|e| format!("Failed to load review: {e}"))?
                .ok_or_else(|| "Review not found".to_string())?;

            let (owner, repo, number, head_sha) = match &review.source {
                crate::domain::ReviewSource::GitHubPr {
                    owner,
                    repo,
                    number,
                    head_sha,
                    ..
                } => (owner.clone(), repo.clone(), *number, head_sha.clone()),
                _ => return Err("Review is not a GitHub PR".to_string()),
            };

            let commit_id =
                head_sha.ok_or_else(|| "Missing PR head SHA; re-run PR fetch".to_string())?;

            let mut links = Vec::new();
            let mut summary_url = None;

            if include_summary {
                let run_id = review
                    .active_run_id
                    .clone()
                    .ok_or_else(|| "No active run for this review; cannot build summary.".to_string())?;
                let run = run_repo
                    .find_by_id(&run_id)
                    .map_err(|e| format!("Failed to load run: {e}"))?
                    .ok_or_else(|| "Run not found for review.".to_string())?;
                let tasks = task_repo
                    .find_by_run(&run_id)
                    .map_err(|e| format!("Failed to load tasks: {e}"))?;
                let feedbacks = feedback_repo
                    .find_by_review(&review.id)
                    .map_err(|e| format!("Failed to load feedbacks: {e}"))?;
                let mut comments = Vec::new();
                for f in &feedbacks {
                    let mut feedback_comments = comment_repo
                        .list_for_feedback(&f.id)
                        .map_err(|e| format!("Failed to load feedback comments: {e}"))?;
                    comments.append(&mut feedback_comments);
                }

                let export_data = ExportData {
                    review: review.clone(),
                    run: run.clone(),
                    tasks,
                    feedbacks: feedbacks.clone(),
                    comments,
                };
                let export_options = ExportOptions {
                    include_summary: true,
                    include_stats: true,
                    include_metadata: false,
                    include_tasks: true,
                    include_feedbacks: false,
                    include_feedback_ids: None,
                };

                let summary = ReviewExporter::export_to_markdown(&export_data, &export_options)
                    .await
                    .map_err(|e| format!("Failed to render summary: {e}"))?;

                let posted = crate::infra::vcs::github::create_review(
                    &owner,
                    &repo,
                    number,
                    &summary.markdown,
                )
                .await
                .map_err(|e| format!("Failed to post summary review: {e}"))?;
                summary_url = posted.url;
            }

            for feedback_id in feedback_ids {
                let feedback = feedback_repo
                    .find_by_id(&feedback_id)
                    .map_err(|e| format!("Failed to load feedback: {e}"))?
                    .ok_or_else(|| "Feedback not found".to_string())?;

                if feedback.review_id != review.id {
                    return Err("Feedback does not belong to selected review".to_string());
                }

                let anchor = feedback.anchor.as_ref().ok_or_else(|| {
                    "Feedback missing anchor; open it from the diff to attach a location."
                        .to_string()
                })?;
                let path = anchor
                    .file_path
                    .as_deref()
                    .ok_or_else(|| "Feedback missing file path".to_string())?;
                let line = anchor
                    .line_number
                    .ok_or_else(|| "Feedback missing line number".to_string())?;
                let side = anchor.side.ok_or_else(|| {
                    "Feedback missing side; add the feedback from the diff again.".to_string()
                })?;

                let task_id = feedback
                    .task_id
                    .clone()
                    .ok_or_else(|| "Feedback is not linked to a task; open it from a task to send it to the PR.".to_string())?;
                let task = task_repo
                    .find_by_id(&task_id)
                    .map_err(|e| format!("Failed to load task: {e}"))?
                    .ok_or_else(|| "Task not found for feedback; refresh the review data.".to_string())?;
                let run = run_repo
                    .find_by_id(&task.run_id)
                    .map_err(|e| format!("Failed to load run: {e}"))?
                    .ok_or_else(|| "Run not found for task; refresh the review data.".to_string())?;

                let position = compute_diff_position(&run.diff_text, path, line, side)?;

                let comments = comment_repo
                    .list_for_feedback(&feedback.id)
                    .map_err(|e| format!("Failed to load feedback comments: {e}"))?;

                let body = ReviewExporter::render_single_feedback_markdown(&feedback, &comments);

                let posted = crate::infra::vcs::github::create_review_comment(
                    &owner,
                    &repo,
                    number,
                    &body,
                    &commit_id,
                    path,
                    position,
                )
                .await
                .map_err(|e| format!("Failed to post review comment: {e}"))?;

                let link = crate::domain::FeedbackLink {
                    id: uuid::Uuid::new_v4().to_string(),
                    feedback_id: feedback.id.clone(),
                    provider: "github".to_string(),
                    provider_feedback_id: posted.id.clone(),
                    provider_root_comment_id: posted.url.clone().unwrap_or_else(|| posted.id.clone()),
                    last_synced_at: chrono::Utc::now().to_rfc3339(),
                };

                feedback_link_repo
                    .save(&link)
                    .map_err(|e| format!("Failed to save feedback link: {e}"))?;

                links.push(link);
            }

            Ok(SendToPrResult { links, summary_url })
        }
        .await;

        let _ = action_tx
            .send(Action::Async(AsyncAction::SendToPrFinished(result)))
            .await;
    });
}

fn compute_diff_position(
    diff_text: &str,
    anchor_path: &str,
    line_number: u32,
    side: FeedbackSide,
) -> Result<u32, String> {
    let trimmed = diff_text.trim();
    if trimmed.is_empty() {
        return Err("PR diff is empty; refresh the PR diff before sending.".to_string());
    }

    let mut patch = PatchSet::new();
    patch
        .parse(trimmed)
        .map_err(|e| format!("Failed to parse PR diff: {e}"))?;

    let normalize = |path: &str| {
        path.strip_prefix("a/")
            .or_else(|| path.strip_prefix("b/"))
            .unwrap_or(path)
            .to_string()
    };
    let basename = |path: &str| path.rsplit('/').next().unwrap_or(path).to_string();

    let normalized_anchor = normalize(anchor_path);
    let anchor_basename = basename(&normalized_anchor);

    let paths_match = |diff_path: &str| -> bool {
        if diff_path == normalized_anchor {
            return true;
        }
        if diff_path.ends_with(&format!("/{}", normalized_anchor)) {
            return true;
        }
        if normalized_anchor.ends_with(&format!("/{}", diff_path)) {
            return true;
        }
        basename(diff_path) == anchor_basename
    };

    for file in patch.files() {
        let target = normalize(&file.target_file);
        let source = normalize(&file.source_file);
        if !paths_match(&target) && !paths_match(&source) {
            continue;
        }

        let mut position: u32 = 0;
        for hunk in file.hunks() {
            // Count the hunk header line.
            position += 1;
            for line in hunk.lines() {
                position += 1;
                let matches_line = match side {
                    FeedbackSide::New => line.target_line_no == Some(line_number as usize),
                    FeedbackSide::Old => line.source_line_no == Some(line_number as usize),
                };
                if !matches_line {
                    continue;
                }

                let matches_side = match side {
                    FeedbackSide::New => line.is_added() || line.is_context(),
                    FeedbackSide::Old => line.is_removed() || line.is_context(),
                };

                if matches_side {
                    return Ok(position);
                }
            }
        }

        return Err(format!(
            "Line {} not found in diff for {}; verify the feedback line still exists in the PR.",
            line_number, anchor_path
        ));
    }

    Err(format!(
        "File {} not found in the PR diff; open the feedback from the diff view and retry.",
        anchor_path
    ))
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
pub fn create_feedback_comment(
    app: &mut LaReviewApp,
    review_id: ReviewId,
    task_id: crate::domain::TaskId,
    feedback_id: Option<String>,
    file_path: Option<String>,
    line_number: Option<u32>,
    side: Option<crate::domain::FeedbackSide>,
    title: Option<String>,
    body: String,
) {
    let now = chrono::Utc::now().to_rfc3339();
    let is_new_feedback = feedback_id.is_none();
    let feedback_id = feedback_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let result = (|| -> Result<(), String> {
        if is_new_feedback {
            let title = title.unwrap_or_else(|| default_feedback_title(&body));
            let anchor = if file_path.is_some() || line_number.is_some() {
                Some(FeedbackAnchor {
                    file_path: file_path.clone(),
                    line_number,
                    side,
                    hunk_ref: None,
                    head_sha: app
                        .review_repo
                        .find_by_id(&review_id)
                        .ok()
                        .flatten()
                        .and_then(|r| match r.source {
                            crate::domain::ReviewSource::GitHubPr { head_sha, .. } => head_sha,
                            _ => None,
                        }),
                })
            } else {
                None
            };

            let feedback = Feedback {
                id: feedback_id.clone(),
                review_id: review_id.clone(),
                task_id: Some(task_id.clone()),
                title,
                status: ReviewStatus::Todo,
                impact: FeedbackImpact::Nitpick,
                anchor,
                author: "User".to_string(),
                created_at: now.clone(),
                updated_at: now.clone(),
            };

            app.feedback_repo
                .save(&feedback)
                .map_err(|e| format!("Failed to save feedback: {e}"))?;
        } else {
            app.feedback_repo
                .touch(&feedback_id)
                .map_err(|e| format!("Failed to update feedback timestamp: {e}"))?;
        }

        let comment = Comment {
            id: uuid::Uuid::new_v4().to_string(),
            feedback_id: feedback_id.clone(),
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

    app.dispatch(Action::Async(AsyncAction::FeedbackCommentSaved(
        result.clone(),
    )));

    if result.is_ok() {
        load_review_feedbacks(app, review_id.clone());
        app.dispatch(Action::Review(ReviewAction::OpenFeedback {
            task_id,
            feedback_id: Some(feedback_id),
            file_path,
            line_number,
            side,
        }));
    }
}

pub fn update_feedback_status(app: &mut LaReviewApp, feedback_id: String, status: ReviewStatus) {
    let review_id = app.state.ui.selected_review_id.clone();
    let result = app
        .feedback_repo
        .update_status(&feedback_id, status)
        .map(|_| ())
        .map_err(|e| format!("Failed to update feedback status: {e}"));

    app.dispatch(Action::Async(AsyncAction::FeedbackCommentSaved(
        result.clone(),
    )));

    if result.is_err()
        && let Some(ref _review_id) = review_id
    {
        warn!("[review] Failed to update feedback status, skipping reload");
    }

    if let (Ok(_), Some(review_id)) = (result, review_id) {
        load_review_feedbacks(app, review_id);
    }
}

pub fn update_feedback_impact(app: &mut LaReviewApp, feedback_id: String, impact: FeedbackImpact) {
    let review_id = app.state.ui.selected_review_id.clone();
    let result = app
        .feedback_repo
        .update_impact(&feedback_id, impact)
        .map(|_| ())
        .map_err(|e| format!("Failed to update feedback impact: {e}"));

    app.dispatch(Action::Async(AsyncAction::FeedbackCommentSaved(
        result.clone(),
    )));

    if result.is_err()
        && let Some(ref _review_id) = review_id
    {
        warn!("[review] Failed to update feedback impact, skipping reload");
    }

    if let (Ok(_), Some(review_id)) = (result, review_id) {
        load_review_feedbacks(app, review_id);
    }
}

pub fn update_feedback_title(app: &mut LaReviewApp, feedback_id: String, title: String) {
    let review_id = app.state.ui.selected_review_id.clone();
    let result = app
        .feedback_repo
        .update_title(&feedback_id, &title)
        .map(|_| ())
        .map_err(|e| format!("Failed to update feedback title: {e}"));

    app.dispatch(Action::Async(AsyncAction::FeedbackCommentSaved(
        result.clone(),
    )));

    if result.is_err()
        && let Some(ref _review_id) = review_id
    {
        warn!("[review] Failed to update feedback title, skipping reload");
    }

    if let (Ok(_), Some(review_id)) = (result, review_id) {
        load_review_feedbacks(app, review_id);
    }
}

pub fn delete_feedback(app: &mut LaReviewApp, feedback_id: String) {
    let review_id = app.state.ui.selected_review_id.clone();
    let result = app
        .feedback_repo
        .delete(&feedback_id)
        .map(|_| ())
        .map_err(|e| format!("Failed to delete feedback: {e}"));

    if result.is_err()
        && let Some(ref _review_id) = review_id
    {
        warn!("[review] Failed to delete feedback, skipping reload");
    }

    if let (Ok(_), Some(review_id)) = (&result, review_id) {
        load_review_feedbacks(app, review_id);
    }
}

pub fn delete_comment(app: &mut LaReviewApp, comment_id: String) {
    let review_id = app.state.ui.selected_review_id.clone();
    let result = app
        .comment_repo
        .delete(&comment_id)
        .map(|_| ())
        .map_err(|e| format!("Failed to delete comment: {e}"));

    if result.is_err()
        && let Some(ref _review_id) = review_id
    {
        warn!("[review] Failed to delete comment, skipping reload");
    }

    if let (Ok(_), Some(review_id)) = (&result, review_id) {
        load_review_feedbacks(app, review_id);
    }
}

pub fn generate_export_preview(
    app: &mut LaReviewApp,
    review_id: crate::domain::ReviewId,
    run_id: crate::domain::ReviewRunId,
    include_feedback_ids: Option<Vec<String>>,
    options: crate::application::review::export::ExportOptions,
) {
    let review_repo = app.review_repo.clone();
    let run_repo = app.run_repo.clone();
    let task_repo = app.task_repo.clone();
    let feedback_repo = app.feedback_repo.clone();
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

            let mut feedbacks = feedback_repo
                .find_by_review(&review_id)
                .map_err(|e: anyhow::Error| e.to_string())?;

            if let Some(include_ids) = &include_feedback_ids {
                feedbacks.retain(|t| include_ids.contains(&t.id));
            }

            let mut comments = Vec::new();
            for feedback in &feedbacks {
                let mut feedback_comments = comment_repo
                    .list_for_feedback(&feedback.id)
                    .map_err(|e: anyhow::Error| e.to_string())?;
                comments.append(&mut feedback_comments);
            }

            let data = crate::application::review::export::ExportData {
                review,
                run,
                tasks,
                feedbacks,
                comments,
            };

            crate::application::review::export::ReviewExporter::export_to_markdown(&data, &options)
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
    options: crate::application::review::export::ExportOptions,
) {
    let review_repo = app.review_repo.clone();
    let run_repo = app.run_repo.clone();
    let task_repo = app.task_repo.clone();
    let feedback_repo = app.feedback_repo.clone();
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

            let feedbacks = feedback_repo
                .find_by_review(&review_id)
                .map_err(|e: anyhow::Error| anyhow::anyhow!(e))?;
            let mut comments = Vec::new();
            for feedback in &feedbacks {
                let mut feedback_comments = comment_repo
                    .list_for_feedback(&feedback.id)
                    .map_err(|e: anyhow::Error| anyhow::anyhow!(e))?;
                comments.append(&mut feedback_comments);
            }

            let data = crate::application::review::export::ExportData {
                review,
                run,
                tasks,
                feedbacks,
                comments,
            };

            let export_result =
                crate::application::review::export::ReviewExporter::export_to_markdown(
                    &data, &options,
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

fn default_feedback_title(body: &str) -> String {
    let first_line = body.lines().next().unwrap_or("").trim();
    if first_line.is_empty() {
        return "Untitled feedback".to_string();
    }
    if first_line.len() > 80 {
        format!("{}...", &first_line[..77])
    } else {
        first_line.to_string()
    }
}
