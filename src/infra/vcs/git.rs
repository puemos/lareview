use std::path::Path;
use std::process::Command;

/// Compare two locally available commits without depending on a hosting service.
pub struct LocalGitDiff {
    pub diff_text: String,
    pub base_sha: String,
    pub head_sha: String,
}

pub async fn compare_refs(
    repo_path: &Path,
    base: &str,
    head: &str,
) -> anyhow::Result<LocalGitDiff> {
    use anyhow::{Context, bail};

    let git = crate::infra::shell::find_bin("git").context("Could not find 'git' executable")?;
    let command = || {
        let mut command = tokio::process::Command::new(&git);
        command
            .arg("-C")
            .arg(repo_path)
            .env("LC_ALL", "C")
            .env("GIT_NO_LAZY_FETCH", "1")
            .kill_on_drop(true);
        command
    };

    let mut commits = Vec::new();
    for reference in [base, head] {
        let reference = reference.trim();
        if reference.is_empty() || reference.starts_with('-') {
            bail!("Enter a branch, tag, or commit for both refs");
        }
        let revision = format!("{reference}^{{commit}}");
        let output = command()
            .args(["rev-parse", "--verify", "--end-of-options", &revision])
            .output()
            .await
            .context("Failed to resolve Git ref")?;
        if !output.status.success() {
            bail!(
                "Could not resolve ref '{reference}'. Fetch it with git first, then compare again."
            );
        }
        commits.push(String::from_utf8(output.stdout)?.trim().to_string());
    }

    let output = command()
        .args([
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            &commits[0],
            &commits[1],
            "--",
        ])
        .output()
        .await
        .context("Failed to run git diff")?;
    if !output.status.success() {
        bail!(
            "git diff failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let diff_text = String::from_utf8(output.stdout)?;
    if diff_text.trim().is_empty() {
        bail!("No changes between '{base}' and '{head}'");
    }
    Ok(LocalGitDiff {
        diff_text,
        base_sha: commits.remove(0),
        head_sha: commits.remove(0),
    })
}

/// Extract remote URLs from a git repository
pub fn extract_git_remotes(repo_path: &Path) -> Vec<String> {
    let output = Command::new("git")
        .args(["-C", &repo_path.to_string_lossy(), "remote", "-v"])
        .output()
        .ok();

    let mut remotes = Vec::new();
    if let Some(output) = output
        && output.status.success()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let url = parts[1].to_string();
                if !remotes.contains(&url) {
                    remotes.push(url);
                }
            }
        }
    }
    remotes
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_extract_git_remotes_empty() {
        let dir = tempdir().unwrap();
        // Not a git repo, or at least no remotes
        let remotes = extract_git_remotes(dir.path());
        assert!(remotes.is_empty());
    }

    #[test]
    fn test_extract_git_remotes_mock() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path();

        // Initialize a git repo and add a remote
        let status = std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .status()
            .unwrap();
        if !status.success() {
            return; // Skip if git is not installed or failed
        }

        std::process::Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "https://github.com/example/repo.git",
            ])
            .current_dir(repo_path)
            .status()
            .unwrap();

        let remotes = extract_git_remotes(repo_path);
        assert_eq!(remotes.len(), 1);
        assert_eq!(remotes[0], "https://github.com/example/repo.git");
    }
}
