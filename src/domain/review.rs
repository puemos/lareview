use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Unique identifier for a pull request
pub type ReviewId = String;

/// Unique identifier for a review generation run
pub type ReviewRunId = String;

/// A review to be generated and tracked in the app.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    /// Unique identifier for the review.
    pub id: ReviewId,
    /// Agent-generated title (GitHub PR title may be used as initial title).
    pub title: String,
    /// Optional agent-generated summary of the review.
    pub summary: Option<String>,
    /// Where this review came from (diff paste or GitHub PR).
    pub source: ReviewSource,
    /// Latest run ID currently shown in the Review UI.
    pub active_run_id: Option<ReviewRunId>,
    /// Current status of the review.
    #[serde(default)]
    pub status: crate::domain::ReviewStatus,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Source for a review.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReviewSource {
    /// Review is based purely on a pasted diff.
    DiffPaste {
        /// Hash of the unified diff used for the run that created this review.
        diff_hash: String,
    },
    /// Review is derived from a GitHub pull request fetched locally via `gh`.
    #[serde(rename = "github_pr", alias = "git_hub_pr")]
    GitHubPr {
        /// GitHub owner (organization or user)
        owner: String,
        /// GitHub repository name
        repo: String,
        /// Pull request number
        number: u32,
        /// Optional canonical URL for the PR
        #[serde(default)]
        url: Option<String>,
        /// Topmost commit SHA of the PR
        #[serde(default)]
        head_sha: Option<String>,
        /// Base commit SHA of the target branch
        #[serde(default)]
        base_sha: Option<String>,
    },
}

impl ReviewSource {
    pub fn url(&self) -> Option<String> {
        match self {
            ReviewSource::DiffPaste { .. } => None,
            ReviewSource::GitHubPr { url, .. } => url.clone(),
        }
    }

    pub fn head_sha(&self) -> Option<String> {
        match self {
            ReviewSource::DiffPaste { .. } => None,
            ReviewSource::GitHubPr { head_sha, .. } => head_sha.clone(),
        }
    }
}

/// A single generation run for a review (diff + agent output).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRun {
    /// Unique identifier for the run.
    pub id: ReviewRunId,
    /// Parent review.
    pub review_id: ReviewId,
    /// Agent id used for this run.
    pub agent_id: String,
    /// Raw user input reference (diff paste text or PR URL/ID).
    pub input_ref: String,
    /// Canonical unified diff used for this run.
    pub diff_text: Arc<str>,
    /// Hash of `diff_text` for quick change checks/dedupe.
    pub diff_hash: String,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
}
