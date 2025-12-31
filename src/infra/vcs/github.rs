use crate::infra::shell;
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct GitHubPrRef {
    pub owner: String,
    pub repo: String,
    pub number: u32,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct GitHubPrMetadata {
    pub title: String,
    pub url: String,
    pub head_sha: Option<String>,
    pub base_sha: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GitHubReviewComment {
    pub id: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GitHubReview {
    pub id: String,
    pub url: Option<String>,
}

lazy_static! {
    static ref GH_PR_URL_RE: Regex =
        Regex::new(r"^https?://(?:www\.)?github\.com/([^/]+)/([^/]+)/pull/(\d+)")
            .expect("github pr url regex");
    static ref GH_OWNER_REPO_NUM_RE: Regex =
        Regex::new(r"^([^/\s]+)/([^#\s]+)#(\d+)$").expect("github owner/repo#num regex");
}

pub fn parse_pr_ref(input: &str) -> Option<GitHubPrRef> {
    let trimmed = input.trim();
    if let Some(caps) = GH_PR_URL_RE.captures(trimmed) {
        let owner = caps.get(1)?.as_str().to_string();
        let repo = caps.get(2)?.as_str().to_string();
        let number: u32 = caps.get(3)?.as_str().parse().ok()?;
        let url = format!("https://github.com/{owner}/{repo}/pull/{number}");
        return Some(GitHubPrRef {
            owner,
            repo,
            number,
            url,
        });
    }

    if let Some(caps) = GH_OWNER_REPO_NUM_RE.captures(trimmed) {
        let owner = caps.get(1)?.as_str().to_string();
        let repo = caps.get(2)?.as_str().to_string();
        let number: u32 = caps.get(3)?.as_str().parse().ok()?;
        let url = format!("https://github.com/{owner}/{repo}/pull/{number}");
        return Some(GitHubPrRef {
            owner,
            repo,
            number,
            url,
        });
    }

    None
}

#[derive(Debug, Deserialize)]
struct GhPrViewJson {
    title: String,
    url: String,
    #[serde(rename = "headRefOid")]
    head_ref_oid: Option<String>,
    #[serde(rename = "baseRefOid")]
    base_ref_oid: Option<String>,
}

pub async fn fetch_pr_metadata(pr: &GitHubPrRef) -> Result<GitHubPrMetadata> {
    let gh_path = shell::find_bin("gh").context("resolve `gh` path")?;
    let output = Command::new(&gh_path)
        .args([
            "pr",
            "view",
            pr.url.as_str(),
            "--json",
            "title,url,headRefOid,baseRefOid",
        ])
        .output()
        .await
        .context("run `gh pr view`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(format!("`gh pr view` failed: {stderr}")));
    }

    let json = String::from_utf8(output.stdout).context("decode `gh pr view` stdout")?;
    let parsed: GhPrViewJson = serde_json::from_str(&json).context("parse `gh pr view` json")?;

    Ok(GitHubPrMetadata {
        title: parsed.title,
        url: parsed.url,
        head_sha: parsed.head_ref_oid,
        base_sha: parsed.base_ref_oid,
    })
}

pub async fn fetch_pr_diff(pr: &GitHubPrRef) -> Result<String> {
    let gh_path = shell::find_bin("gh").context("resolve `gh` path")?;
    let output = Command::new(&gh_path)
        .args(["pr", "diff", pr.url.as_str()])
        .output()
        .await
        .context("run `gh pr diff`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(format!("`gh pr diff` failed: {stderr}")));
    }

    String::from_utf8(output.stdout).context("decode `gh pr diff` stdout")
}

fn normalize_repo_path(path: &str) -> String {
    path.strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .unwrap_or(path)
        .to_string()
}

/// Post a single review comment at a diff position. This creates a review thread automatically.
pub async fn create_review_comment(
    owner: &str,
    repo: &str,
    number: u32,
    body: &str,
    commit_id: &str,
    path: &str,
    position: u32,
) -> Result<GitHubReviewComment> {
    let gh_path = shell::find_bin("gh").context("resolve `gh` path")?;
    let normalized_path = normalize_repo_path(path);

    let payload = serde_json::json!({
        "body": body,
        "commit_id": commit_id,
        "path": normalized_path,
        "position": position,
    });

    let mut child = Command::new(&gh_path)
        .args([
            "api",
            &format!("repos/{owner}/{repo}/pulls/{number}/comments"),
            "--method",
            "POST",
            "-H",
            "Accept: application/vnd.github+json",
            "--input",
            "-",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("spawn `gh api` for pull review")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(payload.to_string().as_bytes())
            .await
            .context("write payload to gh stdin")?;
    }

    let output = child
        .wait_with_output()
        .await
        .context("run `gh api` to create review comment")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(format!("`gh api` failed: {stderr}")));
    }

    let json = String::from_utf8(output.stdout).context("decode `gh api` stdout")?;
    let parsed: serde_json::Value =
        serde_json::from_str(&json).context("parse `gh api` response json")?;

    let id = parsed
        .get("id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| anyhow::anyhow!("Missing comment id in GitHub response"))?
        .to_string();
    let url = parsed
        .get("html_url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(GitHubReviewComment { id, url })
}

/// Create a PR review with a single overall comment.
pub async fn create_review(
    owner: &str,
    repo: &str,
    number: u32,
    body: &str,
) -> Result<GitHubReview> {
    let gh_path = shell::find_bin("gh").context("resolve `gh` path")?;

    let payload = serde_json::json!({
        "event": "COMMENT",
        "body": body,
    });

    let mut child = Command::new(&gh_path)
        .args([
            "api",
            &format!("repos/{owner}/{repo}/pulls/{number}/reviews"),
            "--method",
            "POST",
            "-H",
            "Accept: application/vnd.github+json",
            "--input",
            "-",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("spawn `gh api` for pull review creation")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(payload.to_string().as_bytes())
            .await
            .context("write payload to gh stdin")?;
    }

    let output = child
        .wait_with_output()
        .await
        .context("run `gh api` to create review")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(format!("`gh api` failed: {stderr}")));
    }

    let json = String::from_utf8(output.stdout).context("decode `gh api` stdout")?;
    let parsed: serde_json::Value =
        serde_json::from_str(&json).context("parse `gh api` response json")?;

    let id = parsed
        .get("id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| anyhow::anyhow!("Missing review id in GitHub response"))?
        .to_string();
    let url = parsed
        .get("html_url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(GitHubReview { id, url })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pr_ref_valid_url() {
        let input = "https://github.com/puemos/lareview/pull/123";
        let res = parse_pr_ref(input).expect("should parse");
        assert_eq!(res.owner, "puemos");
        assert_eq!(res.repo, "lareview");
        assert_eq!(res.number, 123);
    }

    #[test]
    fn test_parse_pr_ref_valid_short_ref() {
        let input = "puemos/lareview#123";
        let res = parse_pr_ref(input).expect("should parse");
        assert_eq!(res.owner, "puemos");
        assert_eq!(res.repo, "lareview");
        assert_eq!(res.number, 123);
    }

    #[test]
    fn test_parse_pr_ref_invalid() {
        assert!(parse_pr_ref("invalid").is_none());
        assert!(parse_pr_ref("https://google.com").is_none());
        assert!(parse_pr_ref("owner/repo").is_none());
    }
}
