use crate::domain::ReviewStatus;
use crate::infra::vcs::registry::VcsRegistry;
use crate::infra::vcs::traits::{FeedbackPushRequest, ReviewPushRequest, VcsStatus};
use crate::state::AppState;
use tauri::State;

use super::diff::{ParsedDiff, parse_diff};

#[tauri::command]
pub async fn fetch_remote_pr(
    _state: State<'_, AppState>,
    pr_ref: String,
    provider_hint: Option<String>,
) -> Result<ParsedDiff, String> {
    let registry = VcsRegistry::default();
    let provider = if let Some(hint) = provider_hint
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        let hint = hint.to_lowercase();
        registry
            .get_provider(&hint)
            .ok_or_else(|| format!("Unknown VCS provider: {}", hint))?
    } else {
        registry
            .detect_provider(&pr_ref)
            .ok_or_else(|| format!("Unsupported VCS reference: {}", pr_ref))?
    };

    let reference = provider
        .parse_ref(&pr_ref)
        .ok_or_else(|| format!("Invalid VCS reference: {}", pr_ref))?;

    let data = provider
        .fetch_pr(reference.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut parsed = parse_diff(data.diff_text)?;
    parsed.title = Some(data.title.clone());
    parsed.source = Some(data.source);

    Ok(parsed)
}

#[tauri::command]
pub fn get_github_token() -> Result<Option<String>, String> {
    Ok(None)
}

#[tauri::command]
pub fn set_github_token(_token: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn get_vcs_status() -> Result<Vec<VcsStatus>, String> {
    let registry = VcsRegistry::default();
    let mut statuses = Vec::new();
    for provider in registry.providers() {
        let status = provider.get_status().await.map_err(|e| e.to_string())?;
        statuses.push(status);
    }
    Ok(statuses)
}

#[tauri::command]
pub async fn get_single_vcs_status(provider_id: String) -> Result<VcsStatus, String> {
    let registry = VcsRegistry::default();
    let provider = registry
        .get_provider(&provider_id)
        .ok_or_else(|| format!("Provider {} not found", provider_id))?;

    let status = provider.get_status().await.map_err(|e| e.to_string())?;
    Ok(status)
}

#[tauri::command]
pub async fn push_remote_review(
    state: State<'_, AppState>,
    review_id: String,
    selected_tasks: Vec<String>,
    selected_feedbacks: Vec<String>,
) -> Result<String, String> {
    let data = {
        let db = state.db.lock().map_err(|e| e.to_string())?;

        let review = db
            .get_review(&review_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Review not found".to_string())?;

        let active_run_id = review
            .active_run_id
            .clone()
            .ok_or_else(|| "Review has no active run".to_string())?;

        let run = db
            .get_review_run_by_id(&active_run_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Review run not found".to_string())?;

        let tasks = db
            .get_tasks_by_run(&active_run_id)
            .map_err(|e| e.to_string())?;

        let feedbacks = db
            .get_feedback_by_review(&review_id)
            .map_err(|e| e.to_string())?
            // Filter out ignored feedbacks from push to remote
            .into_iter()
            .filter(|f| f.status != ReviewStatus::Ignored)
            .collect::<Vec<_>>();

        let mut comments = Vec::new();
        for f in &feedbacks {
            let f_comments = db
                .get_comments_for_feedback(&f.id)
                .map_err(|e| e.to_string())?;
            comments.extend(f_comments);
        }

        // Fetch merge confidence
        let merge_confidence = db
            .merge_confidence_repo()
            .find_by_run_id(&active_run_id)
            .ok()
            .flatten();

        (review, run, tasks, feedbacks, comments, merge_confidence)
    };

    let request = ReviewPushRequest {
        review: data.0,
        run: data.1,
        tasks: data.2,
        feedbacks: data.3,
        comments: data.4,
        selected_tasks,
        selected_feedbacks,
        merge_confidence: data.5,
    };

    let provider_id = request
        .review
        .source
        .provider_id()
        .ok_or_else(|| "Review has no remote provider".to_string())?;
    let registry = VcsRegistry::default();
    let provider = registry
        .get_provider(provider_id)
        .ok_or_else(|| format!("Unsupported VCS provider: {}", provider_id))?;

    provider
        .push_review(request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn push_remote_feedback(
    state: State<'_, AppState>,
    feedback_id: String,
) -> Result<String, String> {
    let (feedback, review, review_run, comments) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;

        let feedback = db
            .feedback_repo()
            .find_by_id(&feedback_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Feedback not found".to_string())?;

        let review = db
            .get_review(&feedback.review_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Review not found".to_string())?;

        let active_run_id = review
            .active_run_id
            .clone()
            .ok_or_else(|| "Review has no active run".to_string())?;

        let review_run = db
            .get_review_run_by_id(&active_run_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Review run not found".to_string())?;

        let comments = db
            .get_comments_for_feedback(&feedback_id)
            .map_err(|e| e.to_string())?;

        (feedback, review, review_run, comments)
    };

    let request = FeedbackPushRequest {
        review,
        run: review_run,
        feedback,
        comments,
    };

    let provider_id = request
        .review
        .source
        .provider_id()
        .ok_or_else(|| "Review has no remote provider".to_string())?;
    let registry = VcsRegistry::default();
    let provider = registry
        .get_provider(provider_id)
        .ok_or_else(|| format!("Unsupported VCS provider: {}", provider_id))?;

    provider
        .push_feedback(request)
        .await
        .map_err(|e| e.to_string())
}
