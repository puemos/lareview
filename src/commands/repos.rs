use crate::domain::LinkedRepo as DomainLinkedRepo;
use crate::infra::vcs::registry::VcsRegistry;
use crate::infra::vcs::traits::VcsCloneRequest;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn get_linked_repos(state: State<'_, AppState>) -> Result<Vec<LinkedRepoState>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let repos = db.get_linked_repos().map_err(|e| e.to_string())?;
    Ok(repos)
}

#[tauri::command]
pub fn set_repo_snapshot_access(
    state: State<'_, AppState>,
    repo_id: String,
    allowed: bool,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.repo_repo()
        .update_snapshot_access(&repo_id, allowed)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneRepoRequest {
    pub provider: String,
    pub repo: String,
    pub host: Option<String>,
    pub dest_dir: String,
}

fn link_repo_impl(state: &AppState, path: String) -> Result<LinkedRepo, String> {
    let id = Uuid::new_v4().to_string();
    let name = path.split('/').next_back().unwrap_or(&path).to_string();
    let linked_at = chrono::Utc::now().to_rfc3339();

    let domain_repo = DomainLinkedRepo {
        id: id.clone(),
        name: name.clone(),
        path: std::path::PathBuf::from(path.clone()),
        remotes: detect_remotes(&path),
        created_at: linked_at.clone(),
        allow_snapshot_access: false,
    };

    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.repo_repo()
        .save(&domain_repo)
        .map_err(|e| e.to_string())?;

    Ok(LinkedRepo {
        id,
        path,
        name,
        linked_at,
    })
}

#[tauri::command]
pub fn link_repo(state: State<'_, AppState>, path: String) -> Result<LinkedRepo, String> {
    link_repo_impl(state.inner(), path)
}

#[tauri::command]
pub async fn clone_and_link_repo(
    state: State<'_, AppState>,
    request: CloneRepoRequest,
) -> Result<LinkedRepo, String> {
    let provider = request.provider.trim().to_lowercase();
    let repo = request.repo.trim();
    let dest_dir = request.dest_dir.trim();

    if repo.is_empty() {
        return Err("Repository identifier is required".to_string());
    }

    if dest_dir.is_empty() {
        return Err("Destination directory is required".to_string());
    }

    let repo_name = repo.split('/').next_back().unwrap_or(repo);
    let dest_root = std::path::PathBuf::from(dest_dir);
    std::fs::create_dir_all(&dest_root)
        .map_err(|e| format!("Failed to create destination directory: {e}"))?;

    let target_path = dest_root.join(repo_name);
    if target_path.exists() {
        return Err(format!(
            "Destination already exists: {}",
            target_path.display()
        ));
    }

    let registry = VcsRegistry::default();
    let provider = registry
        .get_provider(&provider)
        .ok_or_else(|| format!("Unsupported provider: {}", request.provider))?;

    let clone_request = VcsCloneRequest {
        repo: repo.to_string(),
        dest_path: target_path.clone(),
        host: request.host.clone(),
    };

    provider
        .clone_repo(clone_request)
        .await
        .map_err(|e| e.to_string())?;

    link_repo_impl(state.inner(), target_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn unlink_repo(state: State<'_, AppState>, repo_id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.repo_repo().delete(&repo_id).map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedRepoState {
    pub id: String,
    pub name: String,
    pub path: String,
    pub review_count: usize,
    pub linked_at: String,
    pub remotes: Vec<String>,
    pub allow_snapshot_access: bool,
}

fn detect_remotes(path: &str) -> Vec<String> {
    use std::process::Command;
    let output = Command::new("git")
        .args(["remote", "-v"])
        .current_dir(path)
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut remotes = std::collections::HashSet::new();
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    remotes.insert(parts[1].to_string());
                }
            }
            remotes.into_iter().collect()
        }
        _ => Vec::new(),
    }
}

/// Gets the local repository path for a given review by matching the review's source
/// (GitHub PR or GitLab MR) to a linked local repository via remote URLs.
#[tauri::command]
pub fn get_repo_root_for_review(
    state: State<'_, AppState>,
    review_id: String,
) -> Result<Option<String>, String> {
    use crate::domain::ReviewSource;

    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get the review to access its source
    let review = db
        .get_review(&review_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Review not found: {}", review_id))?;

    // Build the expected remote URL pattern based on source type
    let expected_patterns: Vec<String> = match &review.source {
        ReviewSource::GitHubPr { owner, repo, .. } => {
            vec![
                format!("github.com/{}/{}", owner, repo),
                format!("github.com:{}/{}", owner, repo),
            ]
        }
        ReviewSource::GitLabMr {
            host, project_path, ..
        } => {
            vec![
                format!("{}/{}", host, project_path),
                format!("{}:{}", host, project_path),
            ]
        }

        ReviewSource::DiffPaste { .. } => {
            // For pasted diffs, we can't auto-match to a repo
            return Ok(None);
        }
    };

    // Get all linked repos and search for a match
    let repos = db.get_linked_repos().map_err(|e| e.to_string())?;
    for repo in repos {
        for remote in &repo.remotes {
            // Normalize the remote URL for comparison
            let remote_lower = remote.to_lowercase();
            for pattern in &expected_patterns {
                if remote_lower.contains(&pattern.to_lowercase()) {
                    return Ok(Some(repo.path));
                }
            }
        }
    }

    Ok(None)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedRepo {
    pub id: String,
    pub path: String,
    pub name: String,
    pub linked_at: String,
}
