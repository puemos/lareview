//! Git repository detection utilities.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Detect if current directory is within a git repository
pub fn detect_git_repo() -> Option<PathBuf> {
    let current = std::env::current_dir().ok()?;
    let mut path = current.as_path();

    while path.exists() {
        if path.join(".git").is_dir() {
            return Some(path.to_path_buf());
        }

        // Stop at filesystem root
        if path.parent().is_none() {
            break;
        }

        path = path.parent()?;
    }

    None
}

/// Check if a path is within a git repository
pub fn is_git_repo(path: &Path) -> bool {
    let mut current = Some(path);

    while let Some(p) = current {
        if p.join(".git").is_dir() {
            return true;
        }
        current = p.parent();
    }

    false
}

/// Get the git root directory for a given path
pub fn get_git_root(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path);

    while let Some(p) = current {
        if p.join(".git").is_dir() {
            return Some(p.to_path_buf());
        }
        current = p.parent();
    }

    None
}

/// Extract GitHub repo info from git remotes
pub fn extract_github_info(repo_root: &Path) -> Result<Option<(String, String)>> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(repo_root)
        .output()
        .context("Failed to get git remote")?;

    if !output.status.success() {
        return Ok(None);
    }

    let remote_url = String::from_utf8_lossy(&output.stdout);

    // Parse patterns:
    // - https://github.com/owner/repo.git
    // - git@github.com:owner/repo.git
    // - https://github.com/owner/repo

    if let Some(caps) = regex::Regex::new(r"(?:github\.com[:/]|)([^/]+)/([^/\.]+)")
        .unwrap()
        .captures(&remote_url)
    {
        let owner = caps.get(1).unwrap().as_str().to_string();
        let repo = caps.get(2).unwrap().as_str().to_string();
        // Remove .git suffix if present
        let repo = repo.trim_end_matches(".git").to_string();
        return Ok(Some((owner, repo)));
    }

    Ok(None)
}

/// Get the current git branch
pub fn get_current_branch(repo_root: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_root)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let branch = String::from_utf8_lossy(&output.stdout);
    Some(branch.trim().to_string())
}
