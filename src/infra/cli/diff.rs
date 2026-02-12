//! Diff acquisition from various sources.

use crate::infra::shell;
use crate::infra::vcs::{github, gitlab};
use anyhow::{Context, Result};
use std::io::{IsTerminal, Read};
use std::process::Command;

/// Source of diff input
pub enum DiffSource {
    /// Diff from stdin
    Stdin(String),

    /// Diff between git refs
    GitDiff { from: String, to: String },

    /// GitHub PR (owner/repo#number or full URL)
    GitHubPr {
        owner: String,
        repo: String,
        number: u32,
    },

    /// GitLab MR (group/project!number or full URL)
    GitLabMr {
        host: String,
        project_path: String,
        number: u32,
    },

    /// Current working directory uncommitted changes
    GitStatus,
}

pub enum RemoteRef {
    GitHub {
        owner: String,
        repo: String,
        number: u32,
    },
    GitLab {
        host: String,
        project_path: String,
        number: u32,
    },
}

/// Parse a PR/MR reference into components
pub fn parse_remote_ref(pr_ref: &str) -> Result<RemoteRef> {
    if let Some(res) = github::parse_pr_ref(pr_ref) {
        return Ok(RemoteRef::GitHub {
            owner: res.owner,
            repo: res.repo,
            number: res.number,
        });
    }

    if let Some(res) = gitlab::parse_mr_ref(pr_ref) {
        return Ok(RemoteRef::GitLab {
            host: res.host,
            project_path: res.project_path,
            number: res.number,
        });
    }

    Err(anyhow::anyhow!(
        "Invalid PR reference. Expected owner/repo#number, group/project!number, or URL."
    ))
}

/// Try to read diff from stdin (non-destructive check)
/// Returns Some(diff) if stdin has content, None otherwise
pub fn try_read_stdin_diff() -> Result<Option<String>> {
    if std::io::stdin().is_terminal() {
        return Ok(None);
    }

    let mut buffer = String::new();
    match std::io::stdin().read_to_string(&mut buffer) {
        Ok(0) => Ok(None),
        Ok(_) => Ok(Some(buffer)),
        Err(_) => Ok(None),
    }
}

/// Acquire diff text from stdin (blocking - assumes stdin has content)
pub fn read_stdin_diff() -> Result<String> {
    let mut buffer = String::new();
    std::io::stdin()
        .read_to_string(&mut buffer)
        .context("Failed to read from stdin")?;
    Ok(buffer)
}

/// Acquire diff text from various sources
pub fn acquire_diff(source: DiffSource) -> Result<String> {
    match source {
        DiffSource::Stdin(diff) => Ok(diff),

        DiffSource::GitDiff { from, to } => {
            let git_path = shell::find_bin("git").context("Could not find 'git' executable")?;
            let output = Command::new(git_path)
                .args(["diff", &from, &to])
                .output()
                .context("Failed to run git diff")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);

                if stderr.contains("unknown revision") {
                    anyhow::bail!(
                        "Could not find reference '{}' or '{}'. Run `git branch -a` to see available refs.",
                        from,
                        to
                    );
                }

                anyhow::bail!("git diff failed: {}", stderr);
            }

            let diff = String::from_utf8_lossy(&output.stdout).into_owned();

            if diff.is_empty() {
                anyhow::bail!(
                    "No diff between '{}' and '{}'. The branches may be identical.",
                    from,
                    to
                );
            }

            Ok(diff)
        }

        DiffSource::GitHubPr {
            owner,
            repo,
            number,
        } => {
            let gh_path = shell::find_bin("gh").context("Could not find 'gh' executable")?;
            let output = Command::new(gh_path)
                .args([
                    "pr",
                    "diff",
                    &number.to_string(),
                    "--repo",
                    &format!("{}/{}", owner, repo),
                ])
                .output()
                .context("Failed to fetch PR via gh CLI")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);

                if stderr.contains("Authentication") || stderr.contains("not authenticated") {
                    anyhow::bail!("GitHub authentication required. Run `gh auth login` first.");
                }

                if stderr.contains("Could not resolve to a PR") {
                    anyhow::bail!(
                        "Could not find PR #{}. Check the PR number and repository.",
                        number
                    );
                }

                anyhow::bail!("gh pr diff failed: {}", stderr);
            }

            let diff = String::from_utf8_lossy(&output.stdout).into_owned();

            if diff.is_empty() {
                anyhow::bail!("PR #{} has no changes.", number);
            }

            Ok(diff)
        }

        DiffSource::GitLabMr {
            host,
            project_path,
            number,
        } => {
            let glab_path = shell::find_bin("glab").context("Could not find 'glab' executable")?;
            // Note: `glab mr diff` does not support --hostname (unlike `glab api`).
            // For self-hosted instances we set GITLAB_HOST so glab resolves the right host.
            // Use --raw to get a proper unified diff with `diff --git` headers.
            // Without --raw, glab outputs diffs without file separator headers,
            // causing the parser to only recognise the first file.
            let args = vec![
                "mr".to_string(),
                "diff".to_string(),
                number.to_string(),
                "--raw".to_string(),
                "--repo".to_string(),
                project_path,
            ];

            let mut cmd = Command::new(glab_path);
            cmd.args(args);
            if host != "gitlab.com" {
                cmd.env("GITLAB_HOST", &host);
            }
            let output = cmd.output().context("Failed to fetch MR via glab CLI")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);

                if stderr.contains("Authentication") || stderr.contains("not authenticated") {
                    anyhow::bail!("GitLab authentication required. Run `glab auth login` first.");
                }

                anyhow::bail!("glab mr diff failed: {}", stderr);
            }

            let diff = String::from_utf8_lossy(&output.stdout).into_owned();

            if diff.is_empty() {
                anyhow::bail!("MR !{} has no changes.", number);
            }

            Ok(diff)
        }

        DiffSource::GitStatus => {
            let git_path = shell::find_bin("git").context("Could not find 'git' executable")?;
            let output = Command::new(git_path)
                .args(["diff"])
                .output()
                .context("Failed to run git diff")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("git diff failed: {}", stderr);
            }

            let diff = String::from_utf8_lossy(&output.stdout).into_owned();

            if diff.is_empty() {
                anyhow::bail!("No uncommitted changes. Stage some changes first with `git add`.");
            }

            Ok(diff)
        }
    }
}

/// Acquire diff from git stash
pub fn get_stash_diff(stash_index: usize) -> Result<String> {
    let git_path = shell::find_bin("git").context("Could not find 'git' executable")?;
    let output = Command::new(git_path)
        .args(["stash", "show", "-p", &format!("stash@{{{}}}", stash_index)])
        .output()
        .context("Failed to run git stash show")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git stash show failed: {}", stderr);
    }

    let diff = String::from_utf8_lossy(&output.stdout).into_owned();

    if diff.is_empty() {
        anyhow::bail!("Stash #{} has no changes.", stash_index);
    }

    Ok(diff)
}
