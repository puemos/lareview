use crate::application::review::export::ReviewExporter;
use crate::domain::{FeedbackSide, ReviewSource};
use crate::infra::diff::index::{DiffIndex, LineLocation};
use crate::infra::shell;
use crate::infra::vcs::traits::{
    FeedbackPushRequest, ReviewPushRequest, VcsCloneRequest, VcsCloneResult, VcsPrData,
    VcsProvider, VcsRef, VcsStatus,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct GitLabMrRef {
    pub host: String,
    pub project_path: String,
    pub number: u32,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct GitLabMrMetadata {
    pub title: String,
    pub url: String,
    pub head_sha: Option<String>,
    pub base_sha: Option<String>,
    pub start_sha: Option<String>,
}

lazy_static! {
    static ref GL_MR_URL_RE: Regex = Regex::new(
        r"^(?:(?:https?://)?([^/\s]+))/([^/\s]+(?:/[^/\s]+)*)/-/merge_requests/(\d+)/?$"
    )
    .expect("gitlab mr url regex");
    static ref GL_MR_SHORT_RE: Regex =
        Regex::new(r"^([^\s!#]+(?:/[^\s!#]+)*)[!#](\d+)$").expect("gitlab mr shorthand");
}

pub fn parse_mr_ref(input: &str) -> Option<GitLabMrRef> {
    let trimmed = input.trim();
    if let Some(caps) = GL_MR_URL_RE.captures(trimmed) {
        let host = caps.get(1)?.as_str().to_string();
        let project_path = caps.get(2)?.as_str().to_string();
        let number: u32 = caps.get(3)?.as_str().parse().ok()?;
        let url = format!("https://{host}/{project_path}/-/merge_requests/{number}");
        return Some(GitLabMrRef {
            host,
            project_path,
            number,
            url,
        });
    }

    if let Some(caps) = GL_MR_SHORT_RE.captures(trimmed) {
        let project_path = caps.get(1)?.as_str().to_string();
        let number: u32 = caps.get(2)?.as_str().parse().ok()?;
        let host = "gitlab.com".to_string();
        let url = format!("https://{host}/{project_path}/-/merge_requests/{number}");
        return Some(GitLabMrRef {
            host,
            project_path,
            number,
            url,
        });
    }

    None
}

fn encode_project_path(path: &str) -> String {
    path.replace('/', "%2F")
}

fn glab_args_with_host(host: &str, mut args: Vec<String>) -> Vec<String> {
    if host != "gitlab.com" {
        args.push("--hostname".to_string());
        args.push(host.to_string());
    }
    args
}

#[derive(Debug, Deserialize)]
struct GlabMrJson {
    title: String,
    web_url: String,
    diff_refs: Option<GlabDiffRefs>,
}

#[derive(Debug, Deserialize)]
struct GlabDiffRefs {
    head_sha: Option<String>,
    base_sha: Option<String>,
    start_sha: Option<String>,
}

pub async fn fetch_mr_metadata(mr: &GitLabMrRef) -> Result<GitLabMrMetadata> {
    let glab_path = shell::find_bin("glab").context("resolve `glab` path")?;
    let endpoint = format!(
        "projects/{}/merge_requests/{}",
        encode_project_path(&mr.project_path),
        mr.number
    );
    let args = glab_args_with_host(&mr.host, vec!["api".to_string(), endpoint]);

    let output = Command::new(&glab_path)
        .args(args)
        .output()
        .await
        .context("run `glab api` for MR")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(format!("`glab api` failed: {stderr}")));
    }

    let json = String::from_utf8(output.stdout).context("decode `glab api` stdout")?;
    let parsed: GlabMrJson = serde_json::from_str(&json).context("parse `glab api` json")?;

    Ok(GitLabMrMetadata {
        title: parsed.title,
        url: parsed.web_url,
        head_sha: parsed
            .diff_refs
            .as_ref()
            .and_then(|refs| refs.head_sha.clone()),
        base_sha: parsed
            .diff_refs
            .as_ref()
            .and_then(|refs| refs.base_sha.clone()),
        start_sha: parsed
            .diff_refs
            .as_ref()
            .and_then(|refs| refs.start_sha.clone()),
    })
}

pub async fn fetch_mr_diff(mr: &GitLabMrRef) -> Result<String> {
    let glab_path = shell::find_bin("glab").context("resolve `glab` path")?;
    let args = glab_args_with_host(
        &mr.host,
        vec![
            "mr".to_string(),
            "diff".to_string(),
            mr.number.to_string(),
            "--repo".to_string(),
            mr.project_path.clone(),
        ],
    );

    let output = Command::new(&glab_path)
        .args(args)
        .output()
        .await
        .context("run `glab mr diff`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(format!("`glab mr diff` failed: {stderr}")));
    }

    String::from_utf8(output.stdout).context("decode `glab mr diff` stdout")
}

async fn post_glab_api(
    mr: &GitLabMrRef,
    endpoint: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value> {
    let glab_path = shell::find_bin("glab").context("resolve `glab` path")?;
    let args = glab_args_with_host(
        &mr.host,
        vec![
            "api".to_string(),
            endpoint.to_string(),
            "--method".to_string(),
            "POST".to_string(),
            "--header".to_string(),
            "Content-Type: application/json".to_string(),
            "--input".to_string(),
            "-".to_string(),
        ],
    );

    let mut child = Command::new(&glab_path)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("spawn `glab api`")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(payload.to_string().as_bytes())
            .await
            .context("write payload to glab stdin")?;
    }

    let output = child.wait_with_output().await.context("run `glab api`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(format!("`glab api` failed: {stderr}")));
    }

    let json = String::from_utf8(output.stdout).context("decode `glab api` stdout")?;
    let parsed: serde_json::Value = serde_json::from_str(&json).context("parse glab json")?;
    Ok(parsed)
}

fn mr_ref_from_source(source: &ReviewSource) -> Result<GitLabMrRef> {
    match source {
        ReviewSource::GitLabMr {
            host,
            project_path,
            number,
            url,
            ..
        } => Ok(GitLabMrRef {
            host: host.clone(),
            project_path: project_path.clone(),
            number: *number,
            url: url.clone().unwrap_or_else(|| {
                format!("https://{host}/{project_path}/-/merge_requests/{number}")
            }),
        }),
        _ => Err(anyhow::anyhow!("Review must be from a GitLab MR to push")),
    }
}

fn diff_refs_from_source(source: &ReviewSource) -> Result<(String, String, String)> {
    match source {
        ReviewSource::GitLabMr {
            head_sha,
            base_sha,
            start_sha,
            ..
        } => Ok((
            head_sha
                .clone()
                .ok_or_else(|| anyhow::anyhow!("Missing GitLab head SHA"))?,
            base_sha
                .clone()
                .ok_or_else(|| anyhow::anyhow!("Missing GitLab base SHA"))?,
            start_sha
                .clone()
                .ok_or_else(|| anyhow::anyhow!("Missing GitLab start SHA"))?,
        )),
        _ => Err(anyhow::anyhow!("Review must be from a GitLab MR to push")),
    }
}

fn build_gitlab_position(
    file_path: &str,
    location: &LineLocation,
    base_sha: &str,
    start_sha: &str,
    head_sha: &str,
) -> Result<serde_json::Value> {
    let mut position = serde_json::json!({
        "position_type": "text",
        "base_sha": base_sha,
        "start_sha": start_sha,
        "head_sha": head_sha,
        "new_path": file_path,
        "old_path": file_path,
    });

    let object = position
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Could not build GitLab position"))?;

    if location.is_addition {
        let new_line = location
            .new_line_number
            .ok_or_else(|| anyhow::anyhow!("Missing new line number for addition"))?;
        object.insert("new_line".to_string(), serde_json::json!(new_line));
    } else if location.is_deletion {
        let old_line = location
            .old_line_number
            .ok_or_else(|| anyhow::anyhow!("Missing old line number for deletion"))?;
        object.insert("old_line".to_string(), serde_json::json!(old_line));
    } else {
        if let Some(old_line) = location.old_line_number {
            object.insert("old_line".to_string(), serde_json::json!(old_line));
        }
        if let Some(new_line) = location.new_line_number {
            object.insert("new_line".to_string(), serde_json::json!(new_line));
        }
    }

    Ok(position)
}

pub struct GitLabProvider;

impl GitLabProvider {
    pub fn new() -> Self {
        Self
    }
}

impl VcsRef for GitLabMrRef {
    fn provider_id(&self) -> &str {
        "gitlab"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[async_trait]
impl VcsProvider for GitLabProvider {
    fn id(&self) -> &str {
        "gitlab"
    }

    fn name(&self) -> &str {
        "GitLab"
    }

    fn matches_ref(&self, reference: &str) -> bool {
        let trimmed = reference.trim();
        GL_MR_URL_RE.is_match(trimmed)
            || (trimmed.contains('!') && GL_MR_SHORT_RE.is_match(trimmed))
            || (trimmed.contains("gitlab") && GL_MR_SHORT_RE.is_match(trimmed))
    }

    fn parse_ref(&self, reference: &str) -> Option<Box<dyn VcsRef>> {
        parse_mr_ref(reference).map(|mr| Box::new(mr) as Box<dyn VcsRef>)
    }

    async fn fetch_pr(&self, reference: &dyn VcsRef) -> Result<VcsPrData> {
        let mr = reference
            .as_any()
            .downcast_ref::<GitLabMrRef>()
            .ok_or_else(|| anyhow::anyhow!("Invalid GitLab MR reference"))?;

        let diff_text = fetch_mr_diff(mr).await?;
        let metadata = fetch_mr_metadata(mr).await?;

        Ok(VcsPrData {
            diff_text,
            title: metadata.title.clone(),
            source: ReviewSource::GitLabMr {
                host: mr.host.clone(),
                project_path: mr.project_path.clone(),
                number: mr.number,
                url: Some(metadata.url),
                head_sha: metadata.head_sha,
                base_sha: metadata.base_sha,
                start_sha: metadata.start_sha,
            },
        })
    }

    async fn push_review(&self, request: ReviewPushRequest) -> Result<String> {
        let mr_ref = mr_ref_from_source(&request.review.source)?;
        let (head_sha, base_sha, start_sha) = diff_refs_from_source(&request.review.source)?;
        let diff_index = DiffIndex::new(&request.run.diff_text).ok();
        let mut inline_comments: Vec<(String, LineLocation, String)> = Vec::new();

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

                    let location = diff_index
                        .as_ref()?
                        .find_line_location(&dr.file, line_num, side)?;
                    Some((dr.file.clone(), location))
                });

                if let Some((path, location)) = anchor {
                    inline_comments.push((path, location, body));
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
                    let side = anchor.side.unwrap_or(FeedbackSide::New);
                    if let Some(location) = diff_index
                        .as_ref()
                        .and_then(|idx| idx.find_line_location(path, line_num, side))
                    {
                        inline_comments.push((path.clone(), location, body));
                    }
                }
            }
        }

        let summary_body = format!(
            "# Review: {}\n\n{}",
            request.review.title,
            request.review.summary.as_deref().unwrap_or("")
        )
        .trim()
        .to_string();

        let mut result_url: Option<String> = None;

        if !summary_body.is_empty() {
            let endpoint = format!(
                "projects/{}/merge_requests/{}/notes",
                encode_project_path(&mr_ref.project_path),
                mr_ref.number
            );
            let payload = serde_json::json!({ "body": summary_body });
            let response = post_glab_api(&mr_ref, &endpoint, payload).await?;
            result_url = response
                .get("web_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }

        for (path, location, body) in inline_comments {
            let position =
                build_gitlab_position(&path, &location, &base_sha, &start_sha, &head_sha)?;

            let endpoint = format!(
                "projects/{}/merge_requests/{}/discussions",
                encode_project_path(&mr_ref.project_path),
                mr_ref.number
            );
            let payload = serde_json::json!({
                "body": body,
                "position": position,
            });
            let response = post_glab_api(&mr_ref, &endpoint, payload).await?;
            if result_url.is_none() {
                result_url = response
                    .get("web_url")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
        }

        Ok(result_url.unwrap_or_else(|| "Success".to_string()))
    }

    async fn push_feedback(&self, request: FeedbackPushRequest) -> Result<String> {
        let mr_ref = mr_ref_from_source(&request.review.source)?;
        let (head_sha, base_sha, start_sha) = diff_refs_from_source(&request.review.source)?;
        let diff_index = DiffIndex::new(&request.run.diff_text).ok();
        let markdown = ReviewExporter::render_single_feedback_markdown(
            &request.feedback,
            &request.comments,
            None,
        );

        let anchor = request
            .feedback
            .anchor
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Feedback missing anchor"))?;

        let file_path = anchor
            .file_path
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Feedback missing file path"))?;

        let line_number = anchor
            .line_number
            .ok_or_else(|| anyhow::anyhow!("Feedback missing line number"))?;

        let location = diff_index
            .as_ref()
            .and_then(|idx| {
                idx.find_line_location(
                    &file_path,
                    line_number,
                    anchor.side.unwrap_or(FeedbackSide::New),
                )
            })
            .ok_or_else(|| anyhow::anyhow!("Could not find line position in diff"))?;

        let position =
            build_gitlab_position(&file_path, &location, &base_sha, &start_sha, &head_sha)?;

        let endpoint = format!(
            "projects/{}/merge_requests/{}/discussions",
            encode_project_path(&mr_ref.project_path),
            mr_ref.number
        );
        let payload = serde_json::json!({
            "body": markdown,
            "position": position,
        });
        let response = post_glab_api(&mr_ref, &endpoint, payload).await?;
        let url = response
            .get("web_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Success".to_string());

        Ok(url)
    }

    async fn clone_repo(&self, request: VcsCloneRequest) -> Result<VcsCloneResult> {
        let host = request
            .host
            .clone()
            .unwrap_or_else(|| "gitlab.com".to_string());
        let dest = request.dest_path.to_string_lossy().to_string();
        let (command_path, args) = if let Some(glab_path) = shell::find_bin("glab") {
            let mut args = vec!["repo".to_string(), "clone".to_string(), request.repo, dest];
            if host != "gitlab.com" {
                args.push("--hostname".to_string());
                args.push(host);
            }
            (glab_path, args)
        } else {
            let git_path = shell::find_bin("git").context("resolve `git` path for cloning")?;
            let url = format!("https://{host}/{}.git", request.repo);
            let args = vec!["clone".to_string(), url, dest];
            (git_path, args)
        };

        let output = Command::new(&command_path)
            .args(args)
            .output()
            .await
            .context("run clone command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Clone failed: {stderr}"));
        }

        Ok(VcsCloneResult {
            path: request.dest_path,
        })
    }

    async fn get_status(&self) -> Result<VcsStatus> {
        let glab_path = shell::find_bin("glab");
        match glab_path {
            Some(path) => {
                let path_str = path.to_string_lossy().to_string();
                let output = Command::new(&path)
                    .args(["auth", "status"])
                    .output()
                    .await
                    .context("run `glab auth status`")?;

                let combined_output = format!(
                    "{}\n{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );

                if output.status.success() {
                    let login = combined_output
                        .lines()
                        .find(|line| line.contains("Logged in to") && line.contains(" as "))
                        .and_then(|line| line.split(" as ").nth(1))
                        .map(|value| value.split_whitespace().next().unwrap_or("").to_string());

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
                cli_path: "glab not found".to_string(),
                login: None,
                error: Some("glab executable not found in PATH".to_string()),
            }),
        }
    }
}

impl GitLabMrRef {
    pub fn is_from_short_ref(reference: &str) -> bool {
        GL_MR_SHORT_RE.is_match(reference.trim())
    }
}

#[cfg(test)]
mod tests {
    use super::build_gitlab_position;
    use crate::domain::FeedbackSide;
    use crate::infra::diff::index::DiffIndex;

    #[test]
    fn test_gitlab_position_for_added_line() {
        let diff = r#"--- lib/testo/exams/exam_question.ex
+++ lib/testo/exams/exam_question.ex
@@ -5,7 +5,9 @@ defmodule Testo.Exams.ExamQuestion do
   schema \"exams_questions\" do
     field(:exam_id, :id)
     field(:question_id, :id)
-
+    field(:question_id, :id)
+    field(:question_id, :id)
+    
     timestamps()
   end
"#;

        let index = DiffIndex::new(diff).expect("diff index");
        let location = index
            .find_line_location("lib/testo/exams/exam_question.ex", 8, FeedbackSide::New)
            .expect("line location");
        let position = build_gitlab_position(
            "lib/testo/exams/exam_question.ex",
            &location,
            "base",
            "start",
            "head",
        )
        .expect("position");

        let object = position.as_object().expect("position object");
        assert!(object.get("new_line").is_some());
        assert!(object.get("old_line").is_none());
    }
}
