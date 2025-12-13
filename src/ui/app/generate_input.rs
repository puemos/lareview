use anyhow::{Context, Result};

use crate::domain::Review;
use crate::domain::ReviewSource;
use crate::infra::acp::RunContext;
use crate::infra::github;
use crate::ui::app::messages::GenerateResolvedPayload;
use crate::ui::app::state::{GeneratePreview, GitHubPreview};

fn looks_like_unified_diff(text: &str) -> bool {
    let t = text.trim();
    t.contains("diff --git ")
        || (t.contains("\n--- a/") && t.contains("\n+++ b/"))
        || (t.starts_with("diff --git ") || t.starts_with("--- a/"))
}

pub async fn resolve_generate_input(
    input_text: String,
    selected_agent_id: String,
) -> Result<GenerateResolvedPayload> {
    let trimmed = input_text.trim().to_string();
    if trimmed.is_empty() {
        anyhow::bail!("Input is empty");
    }

    if looks_like_unified_diff(&trimmed) {
        let diff_hash = format!("{:016x}", crate::infra::hash::hash64(&trimmed));
        let review_id = uuid::Uuid::new_v4().to_string();
        let run_id = uuid::Uuid::new_v4().to_string();

        return Ok(GenerateResolvedPayload {
            run_context: RunContext {
                review_id,
                run_id,
                agent_id: selected_agent_id,
                input_ref: trimmed.clone(),
                diff_text: trimmed.clone(),
                diff_hash: diff_hash.clone(),
                source: ReviewSource::DiffPaste {
                    diff_hash: diff_hash.clone(),
                },
                initial_title: None,
                created_at: Some(chrono::Utc::now().to_rfc3339()),
            },
            preview: GeneratePreview {
                diff_text: trimmed,
                github: None,
            },
        });
    }

    let Some(pr_ref) = github::parse_pr_ref(&trimmed) else {
        anyhow::bail!("Input is not a unified diff or a recognized GitHub PR reference");
    };

    let meta = github::fetch_pr_metadata(&pr_ref)
        .await
        .with_context(|| format!("Fetch PR metadata via `gh` ({})", pr_ref.url))?;
    let diff = github::fetch_pr_diff(&pr_ref)
        .await
        .with_context(|| format!("Fetch PR diff via `gh` ({})", pr_ref.url))?;

    let diff_hash = format!("{:016x}", crate::infra::hash::hash64(&diff));
    let review_id = uuid::Uuid::new_v4().to_string();
    let run_id = uuid::Uuid::new_v4().to_string();

    Ok(GenerateResolvedPayload {
        run_context: RunContext {
            review_id,
            run_id,
            agent_id: selected_agent_id,
            input_ref: trimmed,
            diff_text: diff.clone(),
            diff_hash: diff_hash.clone(),
            source: ReviewSource::GitHubPr {
                owner: pr_ref.owner.clone(),
                repo: pr_ref.repo.clone(),
                number: pr_ref.number,
                url: Some(meta.url.clone()),
                head_sha: meta.head_sha.clone(),
                base_sha: meta.base_sha.clone(),
            },
            initial_title: Some(meta.title.clone()),
            created_at: Some(chrono::Utc::now().to_rfc3339()),
        },
        preview: GeneratePreview {
            diff_text: diff,
            github: Some(GitHubPreview { pr: pr_ref, meta }),
        },
    })
}

pub async fn resolve_pr_preview(input_text: String) -> Result<GeneratePreview> {
    let trimmed = input_text.trim().to_string();
    let Some(pr_ref) = github::parse_pr_ref(&trimmed) else {
        anyhow::bail!("Not a recognized GitHub PR reference");
    };

    let meta = github::fetch_pr_metadata(&pr_ref)
        .await
        .with_context(|| format!("Fetch PR metadata via `gh` ({})", pr_ref.url))?;
    let diff = github::fetch_pr_diff(&pr_ref)
        .await
        .with_context(|| format!("Fetch PR diff via `gh` ({})", pr_ref.url))?;

    Ok(GeneratePreview {
        diff_text: diff,
        github: Some(GitHubPreview { pr: pr_ref, meta }),
    })
}

pub async fn resolve_github_refresh(
    review: &Review,
    selected_agent_id: String,
) -> Result<GenerateResolvedPayload> {
    let (owner, repo, number, url) = match &review.source {
        ReviewSource::GitHubPr {
            owner,
            repo,
            number,
            url,
            ..
        } => (
            owner.clone(),
            repo.clone(),
            *number,
            url.clone()
                .unwrap_or_else(|| format!("https://github.com/{owner}/{repo}/pull/{number}")),
        ),
        _ => anyhow::bail!("Review is not a GitHub PR"),
    };

    let pr_ref = github::GitHubPrRef {
        owner,
        repo,
        number,
        url,
    };

    let meta = github::fetch_pr_metadata(&pr_ref)
        .await
        .with_context(|| format!("Fetch PR metadata via `gh` ({})", pr_ref.url))?;
    let diff = github::fetch_pr_diff(&pr_ref)
        .await
        .with_context(|| format!("Fetch PR diff via `gh` ({})", pr_ref.url))?;

    let diff_hash = format!("{:016x}", crate::infra::hash::hash64(&diff));
    let run_id = uuid::Uuid::new_v4().to_string();

    Ok(GenerateResolvedPayload {
        run_context: RunContext {
            review_id: review.id.clone(),
            run_id,
            agent_id: selected_agent_id,
            input_ref: pr_ref.url.clone(),
            diff_text: diff.clone(),
            diff_hash: diff_hash.clone(),
            source: ReviewSource::GitHubPr {
                owner: pr_ref.owner.clone(),
                repo: pr_ref.repo.clone(),
                number: pr_ref.number,
                url: Some(meta.url.clone()),
                head_sha: meta.head_sha.clone(),
                base_sha: meta.base_sha.clone(),
            },
            initial_title: Some(meta.title.clone()),
            created_at: Some(chrono::Utc::now().to_rfc3339()),
        },
        preview: GeneratePreview {
            diff_text: diff,
            github: Some(GitHubPreview { pr: pr_ref, meta }),
        },
    })
}
