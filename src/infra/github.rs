use anyhow::{Context, Result};
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
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
    let output = Command::new("gh")
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
    let output = Command::new("gh")
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
