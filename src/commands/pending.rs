use crate::domain::ReviewSource;
use crate::state::{AppState, PendingDiff};
use serde::{Deserialize, Serialize};
use tauri::State;

#[tauri::command]
pub fn clear_pending_diff(state: State<'_, AppState>) -> Result<(), String> {
    let mut pending = state.pending_diff.lock().map_err(|e| e.to_string())?;
    *pending = None;
    Ok(())
}

#[tauri::command]
pub fn get_pending_reviews(state: State<'_, AppState>) -> Result<Vec<PendingReviewState>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let reviews = db.get_pending_reviews().map_err(|e| e.to_string())?;
    Ok(reviews)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingReviewState {
    pub id: String,
    pub diff: String,
    pub repo_root: Option<String>,
    pub agent: Option<String>,
    pub source: String,
    pub created_at: String,
    #[serde(default)]
    pub review_source: Option<ReviewSource>,
}

#[tauri::command]
pub fn get_pending_review_from_state(
    state: State<'_, AppState>,
) -> Result<Option<PendingReviewState>, String> {
    let pending_diff = state.pending_diff.lock().map_err(|e| e.to_string())?;
    Ok(pending_diff.as_ref().map(|p| PendingReviewState {
        id: uuid::Uuid::new_v4().to_string(),
        diff: p.diff.clone(),
        repo_root: p
            .repo_root
            .as_ref()
            .map(|r| r.to_string_lossy().to_string()),
        agent: p.agent.clone(),
        source: p.source.clone(),
        created_at: p.created_at.to_rfc3339(),
        review_source: None,
    }))
}

#[tauri::command]
pub fn get_diff_request(state: State<'_, AppState>) -> Result<Option<DiffRequestState>, String> {
    let diff_request = state.diff_request.lock().map_err(|e| e.to_string())?;
    Ok(diff_request.as_ref().map(|r| DiffRequestState {
        from: r.from.clone(),
        to: r.to.clone(),
        agent: r.agent.clone(),
        source: r.source.clone(),
    }))
}

#[tauri::command]
pub fn acquire_diff_from_request(state: State<'_, AppState>) -> Result<PendingReviewState, String> {
    let diff_request = {
        let request = state.diff_request.lock().map_err(|e| e.to_string())?;
        request
            .clone()
            .ok_or_else(|| "No diff request found".to_string())?
    };

    let mut review_source: Option<ReviewSource> = None;

    let diff = if let Ok(remote_ref) = crate::infra::cli::diff::parse_remote_ref(&diff_request.from)
    {
        match remote_ref {
            crate::infra::cli::diff::RemoteRef::GitHub {
                owner,
                repo,
                number,
            } => {
                let pr_url = format!("https://github.com/{}/{}/pull/{}", owner, repo, number);
                let pr_ref = crate::infra::vcs::github::GitHubPrRef {
                    owner: owner.clone(),
                    repo: repo.clone(),
                    number,
                    url: pr_url.clone(),
                };
                let metadata = tauri::async_runtime::block_on(
                    crate::infra::vcs::github::fetch_pr_metadata(&pr_ref),
                )
                .ok();

                review_source = Some(ReviewSource::GitHubPr {
                    owner: owner.clone(),
                    repo: repo.clone(),
                    number,
                    url: Some(
                        metadata
                            .as_ref()
                            .map(|metadata| metadata.url.clone())
                            .unwrap_or(pr_url),
                    ),
                    head_sha: metadata
                        .as_ref()
                        .and_then(|metadata| metadata.head_sha.clone()),
                    base_sha: metadata
                        .as_ref()
                        .and_then(|metadata| metadata.base_sha.clone()),
                });

                crate::infra::cli::diff::acquire_diff(
                    crate::infra::cli::diff::DiffSource::GitHubPr {
                        owner,
                        repo,
                        number,
                    },
                )
                .map_err(|e| e.to_string())?
            }
            crate::infra::cli::diff::RemoteRef::GitLab {
                host,
                project_path,
                number,
            } => {
                let url = format!("https://{host}/{project_path}/-/merge_requests/{number}");
                let mr_ref = crate::infra::vcs::gitlab::GitLabMrRef {
                    host: host.clone(),
                    project_path: project_path.clone(),
                    number,
                    url: url.clone(),
                };
                let metadata = tauri::async_runtime::block_on(
                    crate::infra::vcs::gitlab::fetch_mr_metadata(&mr_ref),
                )
                .ok();

                review_source = Some(ReviewSource::GitLabMr {
                    host: host.clone(),
                    project_path: project_path.clone(),
                    number,
                    url: Some(metadata.as_ref().map(|m| m.url.clone()).unwrap_or(url)),
                    head_sha: metadata.as_ref().and_then(|m| m.head_sha.clone()),
                    base_sha: metadata.as_ref().and_then(|m| m.base_sha.clone()),
                    start_sha: metadata.as_ref().and_then(|m| m.start_sha.clone()),
                });

                crate::infra::cli::diff::acquire_diff(
                    crate::infra::cli::diff::DiffSource::GitLabMr {
                        host,
                        project_path,
                        number,
                    },
                )
                .map_err(|e| e.to_string())?
            }
        }
    } else if diff_request.source == "uncommitted changes" {
        crate::infra::cli::diff::acquire_diff(crate::infra::cli::diff::DiffSource::GitStatus)
            .map_err(|e| e.to_string())?
    } else {
        let from = if diff_request.from.is_empty() {
            "HEAD".to_string()
        } else {
            diff_request.from.clone()
        };
        let to = if diff_request.to.is_empty() {
            "HEAD".to_string()
        } else {
            diff_request.to.clone()
        };
        crate::infra::cli::diff::acquire_diff(crate::infra::cli::diff::DiffSource::GitDiff {
            from,
            to,
        })
        .map_err(|e| e.to_string())?
    };

    let repo_root = crate::infra::cli::repo::detect_git_repo();

    let pending = PendingDiff {
        diff,
        repo_root,
        agent: diff_request.agent,
        source: diff_request.source.clone(),
        created_at: chrono::Utc::now(),
    };

    {
        let mut pending_diff = state.pending_diff.lock().map_err(|e| e.to_string())?;
        *pending_diff = Some(pending.clone());
    }

    {
        let mut diff_request = state.diff_request.lock().map_err(|e| e.to_string())?;
        *diff_request = None;
    }

    Ok(PendingReviewState {
        id: uuid::Uuid::new_v4().to_string(),
        diff: pending.diff,
        repo_root: pending.repo_root.map(|r| r.to_string_lossy().to_string()),
        agent: pending.agent,
        source: pending.source,
        created_at: pending.created_at.to_rfc3339(),
        review_source,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffRequestState {
    pub from: String,
    pub to: String,
    pub agent: Option<String>,
    pub source: String,
}
