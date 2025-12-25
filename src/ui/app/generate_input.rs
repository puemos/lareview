use anyhow::{Context, Result};
use std::sync::Arc;

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
    review_id: Option<String>,
) -> Result<GenerateResolvedPayload> {
    let trimmed = input_text.trim().to_string();
    if trimmed.is_empty() {
        anyhow::bail!("Input is empty");
    }

    let review_id = review_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    if looks_like_unified_diff(&trimmed) {
        let diff_hash = format!("{:016x}", crate::infra::hash::hash64(&trimmed));
        let run_id = uuid::Uuid::new_v4().to_string();
        let diff_arc: Arc<str> = Arc::from(trimmed.as_str());

        return Ok(GenerateResolvedPayload {
            run_context: RunContext {
                review_id,
                run_id,
                agent_id: selected_agent_id,
                input_ref: trimmed.clone(),
                diff_text: diff_arc.clone(),
                diff_hash: diff_hash.clone(),
                source: ReviewSource::DiffPaste {
                    diff_hash: diff_hash.clone(),
                },
                initial_title: None,
                created_at: Some(chrono::Utc::now().to_rfc3339()),
            },
            preview: GeneratePreview {
                diff_text: diff_arc,
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
    let run_id = uuid::Uuid::new_v4().to_string();
    let diff_arc: Arc<str> = Arc::from(diff.as_str());

    Ok(GenerateResolvedPayload {
        run_context: RunContext {
            review_id,
            run_id,
            agent_id: selected_agent_id,
            input_ref: trimmed,
            diff_text: diff_arc.clone(),
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
            diff_text: diff_arc,
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
        diff_text: Arc::from(diff.as_str()),
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
    let diff_arc: Arc<str> = Arc::from(diff.as_str());

    Ok(GenerateResolvedPayload {
        run_context: RunContext {
            review_id: review.id.clone(),
            run_id,
            agent_id: selected_agent_id,
            input_ref: pr_ref.url.clone(),
            diff_text: diff_arc.clone(),
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
            diff_text: diff_arc,
            github: Some(GitHubPreview { pr: pr_ref, meta }),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::Mutex;
    static ENV_MUTEX: Mutex<()> = Mutex::const_new(());

    #[tokio::test]
    async fn test_looks_like_unified_diff() {
        let _guard = ENV_MUTEX.lock().await;
        assert!(looks_like_unified_diff(
            "diff --git a/src/main.rs b/src/main.rs"
        ));
        assert!(looks_like_unified_diff("--- a/file.txt\n+++ b/file.txt"));
        assert!(looks_like_unified_diff(
            "\nsome text\n--- a/file.txt\n+++ b/file.txt"
        ));
        assert!(!looks_like_unified_diff("just some text"));
        assert!(!looks_like_unified_diff("owner/repo#123"));
    }

    #[tokio::test]
    async fn test_resolve_generate_input_diff() {
        let _guard = ENV_MUTEX.lock().await;
        let input = "--- a/file.rs\n+++ b/file.rs\n@@ -1,1 +1,1 @@\n-old\n+new".to_string();
        let result = resolve_generate_input(input.clone(), "agent_1".to_string(), None)
            .await
            .unwrap();

        assert_eq!(result.run_context.input_ref, input);
        assert_eq!(result.run_context.agent_id, "agent_1");
        assert!(matches!(
            result.run_context.source,
            ReviewSource::DiffPaste { .. }
        ));
        assert!(result.preview.github.is_none());
    }

    #[tokio::test]
    async fn test_resolve_generate_input_empty() {
        let _guard = ENV_MUTEX.lock().await;
        let result = resolve_generate_input("  ".to_string(), "agent_1".to_string(), None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resolve_generate_input_invalid_github() {
        let _guard = ENV_MUTEX.lock().await;
        let result =
            resolve_generate_input("invalid_ref".to_string(), "agent_1".to_string(), None).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Input is not a unified diff or a recognized GitHub PR reference"
        );
    }

    #[tokio::test]
    async fn test_resolve_pr_preview_invalid() {
        let _guard = ENV_MUTEX.lock().await;
        let result = resolve_pr_preview("invalid".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resolve_github_refresh_invalid() {
        let _guard = ENV_MUTEX.lock().await;
        let review = Review {
            id: "rev1".into(),
            title: "Title".into(),
            summary: None,
            source: ReviewSource::DiffPaste {
                diff_hash: "hash".into(),
            },
            active_run_id: None,
            created_at: "".into(),
            updated_at: "".into(),
        };
        let result = resolve_github_refresh(&review, "agent1".into()).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Review is not a GitHub PR");
    }
}
