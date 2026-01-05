//! Diff acquisition from various sources.

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

    /// Current working directory uncommitted changes
    GitStatus,
}

/// Parse a PR reference into components
pub fn parse_pr_ref(pr_ref: &str) -> Result<(String, String, u32)> {
    // Match patterns:
    // - https://github.com/owner/repo/pull/123
    // - https://github.com/owner/repo/pull/123/
    // - owner/repo#123
    // - owner/repo/pull/123

    if let Some(caps) = regex::Regex::new(r"github\.com[/:]([^/]+)/([^/]+)/pull/(\d+)")
        .unwrap()
        .captures(pr_ref)
    {
        let owner = caps.get(1).unwrap().as_str().to_string();
        let repo = caps.get(2).unwrap().as_str().to_string();
        let number: u32 = caps
            .get(3)
            .unwrap()
            .as_str()
            .parse()
            .context("Invalid PR number")?;
        return Ok((owner, repo, number));
    }

    if let Some(caps) = regex::Regex::new(r"^([^/]+)/([^#]+)#(\d+)$")
        .unwrap()
        .captures(pr_ref)
    {
        let owner = caps.get(1).unwrap().as_str().to_string();
        let repo = caps.get(2).unwrap().as_str().to_string();
        let number: u32 = caps
            .get(3)
            .unwrap()
            .as_str()
            .parse()
            .context("Invalid PR number")?;
        return Ok((owner, repo, number));
    }

    anyhow::bail!(
        "Invalid PR reference: {}\n\nValid formats:\n  - owner/repo#123\n  - https://github.com/owner/repo/pull/123",
        pr_ref
    )
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
            let output = Command::new("git")
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
            let pr_ref = format!("{}/{}/PR{}", owner, repo, number);

            let output = Command::new("gh")
                .args(["pr", "diff", &pr_ref])
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

        DiffSource::GitStatus => {
            let output = Command::new("git")
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
    let output = Command::new("git")
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
