use crate::application::review::candidates::annotate_candidates;
use crate::domain::AnnotatedCandidate;
use crate::domain::ReviewStatus;
use crate::infra::vcs::registry::VcsRegistry;
use crate::infra::vcs::traits::{FeedbackPushRequest, ReviewPushRequest, VcsStatus};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
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

/// Result of listing review candidates.
///
/// Providers are polled concurrently and reported independently: one failing
/// provider must not blank the whole inbox, so its error is returned alongside
/// whatever the others produced rather than replacing it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewCandidatesResult {
    pub candidates: Vec<AnnotatedCandidate>,
    pub errors: Vec<ProviderError>,
    /// Providers that cannot report candidates, so the UI can say so rather than
    /// implying their users have nothing to review.
    pub unsupported_providers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderError {
    pub provider_id: String,
    pub provider_name: String,
    pub message: String,
}

#[tauri::command]
pub async fn list_review_candidates(
    state: State<'_, AppState>,
) -> Result<ReviewCandidatesResult, String> {
    let registry = VcsRegistry::default();

    let (supported, unsupported): (Vec<_>, Vec<_>) = registry
        .providers()
        .into_iter()
        .partition(|provider| provider.supports_review_candidates());

    let unsupported_providers: Vec<String> = unsupported
        .into_iter()
        .map(|provider| provider.name().to_string())
        .collect();

    let results = futures::future::join_all(supported.into_iter().map(|provider| async {
        (
            provider.id().to_string(),
            provider.name().to_string(),
            provider.list_review_candidates().await,
        )
    }))
    .await;

    let mut candidates = Vec::new();
    let mut errors = Vec::new();
    for (provider_id, provider_name, result) in results {
        match result {
            Ok(found) => candidates.extend(found),
            Err(err) => errors.push(ProviderError {
                provider_id,
                provider_name,
                message: err.to_string(),
            }),
        }
    }

    let reviews = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.review_repo().list_all().map_err(|e| e.to_string())?
    };

    let mut annotated = annotate_candidates(candidates, &reviews);
    // Most recently updated first — the inbox is a work queue.
    annotated.sort_by(|a, b| b.candidate.updated_at.cmp(&a.candidate.updated_at));

    Ok(ReviewCandidatesResult {
        candidates: annotated,
        errors,
        unsupported_providers,
    })
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
