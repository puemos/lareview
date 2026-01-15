use crate::application::review::export::ReviewExporter;
use crate::domain::{FeedbackSide, ReviewSource};
use crate::infra::diff::index::DiffIndex;
use crate::infra::shell;
use crate::infra::vcs::traits::{
    FeedbackPushRequest, ReviewPushRequest, VcsPrData, VcsProvider, VcsRef, VcsStatus,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
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

#[derive(Debug, Clone, serde::Serialize)]
pub struct DraftReviewComment {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
    pub body: String,
}

lazy_static! {
    static ref GH_PR_RE: Regex = Regex::new(
        r"^(?:(?:https?://)?(?:www\.)?github\.com/)?([^/\s#]+)/([^/\s#]+)(?:/pull/|/|#)(\d+)/?$"
    )
    .expect("github pr regex");
}

pub fn parse_pr_ref(input: &str) -> Option<GitHubPrRef> {
    let trimmed = input.trim();
    if let Some(caps) = GH_PR_RE.captures(trimmed) {
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
        .args([
            "pr",
            "diff",
            &pr.number.to_string(),
            "--repo",
            &format!("{}/{}", pr.owner, pr.repo),
        ])
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

/// Create a PR review with an optional body and individual comments.
pub async fn create_review(
    owner: &str,
    repo: &str,
    number: u32,
    body: Option<&str>,
    comments: Option<Vec<DraftReviewComment>>,
) -> Result<GitHubReview> {
    let gh_path = shell::find_bin("gh").context("resolve `gh` path")?;

    let mut payload = serde_json::json!({
        "event": "COMMENT",
    });

    if let Some(body) = body {
        payload["body"] = serde_json::json!(body);
    }

    if let Some(comments) = comments {
        payload["comments"] = serde_json::to_value(comments)?;
    }

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

fn pr_ref_from_source(source: &ReviewSource) -> Result<GitHubPrRef> {
    match source {
        ReviewSource::GitHubPr {
            owner,
            repo,
            number,
            ..
        } => Ok(GitHubPrRef {
            owner: owner.clone(),
            repo: repo.clone(),
            number: *number,
            url: source.url().unwrap_or_default(),
        }),
        _ => Err(anyhow::anyhow!("Review must be from a GitHub PR to push")),
    }
}

pub struct GitHubProvider;

impl GitHubProvider {
    pub fn new() -> Self {
        Self
    }
}

impl VcsRef for GitHubPrRef {
    fn provider_id(&self) -> &str {
        "github"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[async_trait]
impl VcsProvider for GitHubProvider {
    fn id(&self) -> &str {
        "github"
    }

    fn name(&self) -> &str {
        "GitHub"
    }

    fn matches_ref(&self, reference: &str) -> bool {
        parse_pr_ref(reference).is_some()
    }

    fn parse_ref(&self, reference: &str) -> Option<Box<dyn VcsRef>> {
        parse_pr_ref(reference).map(|pr| Box::new(pr) as Box<dyn VcsRef>)
    }

    async fn fetch_pr(&self, reference: &dyn VcsRef) -> Result<VcsPrData> {
        let pr = reference
            .as_any()
            .downcast_ref::<GitHubPrRef>()
            .ok_or_else(|| anyhow::anyhow!("Invalid GitHub PR reference"))?;
        let diff_text = fetch_pr_diff(pr).await?;
        let metadata = fetch_pr_metadata(pr).await?;
        Ok(VcsPrData {
            diff_text,
            title: metadata.title.clone(),
            source: ReviewSource::GitHubPr {
                owner: pr.owner.clone(),
                repo: pr.repo.clone(),
                number: pr.number,
                url: Some(metadata.url),
                head_sha: metadata.head_sha,
                base_sha: metadata.base_sha,
            },
        })
    }

    async fn push_review(&self, request: ReviewPushRequest) -> Result<String> {
        let diff_index = DiffIndex::new(&request.run.diff_text).ok();
        let mut gh_comments = Vec::new();

        for task_id in &request.selected_tasks {
            if let Some(task) = request.tasks.iter().find(|t| t.id == *task_id) {
                let body = ReviewExporter::render_task_markdown(task);
                let anchor = task.diff_refs.first().and_then(|dr| {
                    let hunk = dr.hunks.first()?;
                    let line_num = if hunk.new_lines > 0 {
                        hunk.new_start
                    } else {
                        hunk.old_start
                    };
                    let side = if hunk.new_lines > 0 {
                        FeedbackSide::New
                    } else {
                        FeedbackSide::Old
                    };

                    diff_index
                        .as_ref()?
                        .find_position_in_diff(&dr.file, line_num, side)?;
                    Some(dr.file.clone())
                });

                if let Some(path) = anchor {
                    let hunk = task.diff_refs.first().unwrap().hunks.first().unwrap();
                    let line_num = if hunk.new_lines > 0 {
                        hunk.new_start
                    } else {
                        hunk.old_start
                    };
                    let side = if hunk.new_lines > 0 { "RIGHT" } else { "LEFT" };

                    gh_comments.push(DraftReviewComment {
                        path,
                        position: None,
                        line: Some(line_num),
                        side: Some(side.to_string()),
                        body,
                    });
                }
            }
        }

        for feedback_id in &request.selected_feedbacks {
            if let Some(feedback) = request.feedbacks.iter().find(|f| f.id == *feedback_id) {
                let feedback_comments: Vec<_> = request
                    .comments
                    .iter()
                    .filter(|c| c.feedback_id == feedback.id)
                    .cloned()
                    .collect();

                let body = ReviewExporter::render_single_feedback_markdown(
                    feedback,
                    &feedback_comments,
                    None,
                );

                if let Some(anchor) = &feedback.anchor
                    && let (Some(path), Some(line_num)) = (&anchor.file_path, anchor.line_number)
                {
                    let side_enum = anchor.side.unwrap_or(FeedbackSide::New);
                    let side_str = match side_enum {
                        FeedbackSide::New => "RIGHT",
                        FeedbackSide::Old => "LEFT",
                    };

                    if diff_index
                        .as_ref()
                        .and_then(|idx| idx.find_position_in_diff(path, line_num, side_enum))
                        .is_some()
                    {
                        gh_comments.push(DraftReviewComment {
                            path: path.clone(),
                            position: None,
                            line: Some(line_num),
                            side: Some(side_str.to_string()),
                            body,
                        });
                        continue;
                    }
                }
            }
        }

        let pr_ref = pr_ref_from_source(&request.review.source)?;
        let summary_body = format!(
            "# Review: {}\n\n{}",
            request.review.title,
            request.review.summary.as_deref().unwrap_or("")
        );

        let gh_review = create_review(
            &pr_ref.owner,
            &pr_ref.repo,
            pr_ref.number,
            Some(&summary_body),
            Some(gh_comments),
        )
        .await?;

        Ok(gh_review.url.unwrap_or_else(|| "Success".to_string()))
    }

    async fn push_feedback(&self, request: FeedbackPushRequest) -> Result<String> {
        let markdown = ReviewExporter::render_single_feedback_markdown(
            &request.feedback,
            &request.comments,
            None,
        );
        let pr_ref = pr_ref_from_source(&request.review.source)?;

        let anchor = request
            .feedback
            .anchor
            .ok_or_else(|| anyhow::anyhow!("Feedback has no anchor"))?;
        let file_path = anchor
            .file_path
            .ok_or_else(|| anyhow::anyhow!("Feedback anchor has no file path"))?;
        let line_number = anchor
            .line_number
            .ok_or_else(|| anyhow::anyhow!("Feedback anchor has no line number"))?;

        let commit_id = request
            .review
            .source
            .head_sha()
            .ok_or_else(|| anyhow::anyhow!("Could not determine head commit SHA for PR"))?;

        let diff_index = DiffIndex::new(&request.run.diff_text)?;

        let position = diff_index
            .find_position_in_diff(
                &file_path,
                line_number,
                anchor.side.unwrap_or(FeedbackSide::New),
            )
            .ok_or_else(|| anyhow::anyhow!("Could not find line position in diff"))?;

        let comment = create_review_comment(
            &pr_ref.owner,
            &pr_ref.repo,
            pr_ref.number,
            &markdown,
            &commit_id,
            &file_path,
            position as u32,
        )
        .await?;

        Ok(comment.url.unwrap_or_else(|| "Success".to_string()))
    }

    async fn get_status(&self) -> Result<VcsStatus> {
        let gh_path = shell::find_bin("gh");
        match gh_path {
            Some(path) => {
                let path_str = path.to_string_lossy().to_string();
                let output = Command::new(&path)
                    .args(["auth", "status"])
                    .output()
                    .await
                    .context("run `gh auth status`")?;

                let combined_output = format!(
                    "{}\n{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );

                if output.status.success() {
                    let login = combined_output
                        .lines()
                        .find(|line| line.contains("Logged in to github.com"))
                        .and_then(|line| {
                            if line.contains(" as ") {
                                line.split(" as ")
                                    .nth(1)
                                    .map(|s| s.split_whitespace().next().unwrap_or("").to_string())
                            } else if line.contains(" account ") {
                                line.split(" account ")
                                    .nth(1)
                                    .map(|s| s.split_whitespace().next().unwrap_or("").to_string())
                            } else {
                                None
                            }
                        });

                    Ok(VcsStatus {
                        id: self.id().to_string(),
                        name: self.name().to_string(),
                        cli_path: path_str,
                        login,
                        error: None,
                    })
                } else {
                    Ok(VcsStatus {
                        id: self.id().to_string(),
                        name: self.name().to_string(),
                        cli_path: path_str,
                        login: None,
                        error: Some(combined_output),
                    })
                }
            }
            None => Ok(VcsStatus {
                id: self.id().to_string(),
                name: self.name().to_string(),
                cli_path: "gh not found".to_string(),
                login: None,
                error: Some("gh executable not found in PATH".to_string()),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pr_ref_valid_url() {
        let inputs = [
            "https://github.com/puemos/lareview/pull/123",
            "http://github.com/puemos/lareview/pull/123",
            "github.com/puemos/lareview/pull/123",
        ];
        for input in inputs {
            let res = parse_pr_ref(input).unwrap_or_else(|| panic!("should parse {}", input));
            assert_eq!(res.owner, "puemos");
            assert_eq!(res.repo, "lareview");
            assert_eq!(res.number, 123);
        }
    }

    #[test]
    fn test_parse_pr_ref_formats() {
        let cases = [
            ("puemos/lareview#123", 123),
            ("puemos/lareview/123", 123),
            ("puemos/lareview/pull/123", 123),
            ("puemos/hls-downloader/490", 490),
        ];
        for (input, expected_num) in cases {
            let res = parse_pr_ref(input).unwrap_or_else(|| panic!("should parse {}", input));
            assert_eq!(res.owner, "puemos");
            assert_eq!(res.number, expected_num);
        }
    }

    #[test]
    fn test_parse_pr_ref_invalid() {
        assert!(parse_pr_ref("invalid").is_none());
        assert!(parse_pr_ref("owner/repo").is_none());
    }
}
