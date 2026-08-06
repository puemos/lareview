//! Pull/merge requests awaiting review.
//!
//! Candidates are a view of remote state, not an entity. They are never
//! persisted — a candidate becomes a [`Review`](super::Review) only once a
//! review is generated for it.

use super::review::ReviewSource;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Why a pull/merge request is being suggested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateReason {
    /// The current user was explicitly requested as a reviewer.
    ReviewRequested,
    /// The current user is assigned, but not requested as a reviewer.
    Assigned,
}

/// A pull/merge request that the current user could review.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewCandidate {
    /// Identifier of the VCS provider that produced this candidate.
    pub provider_id: String,
    /// Carries the head SHA, which is what makes staleness detectable.
    pub source: ReviewSource,
    pub title: String,
    pub url: String,
    pub author: String,
    /// `owner/name` for GitHub, project path for GitLab.
    pub repo: String,
    pub number: u32,
    pub updated_at: DateTime<Utc>,
    pub reason: CandidateReason,
    pub is_draft: bool,
}

/// How a candidate relates to the reviews already stored locally.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateStatus {
    /// No review exists for this pull request.
    New,
    /// A review exists for the current head commit.
    Reviewed { review_id: String },
    /// A review exists, but the pull request has moved on since.
    Stale {
        review_id: String,
        /// Head SHA the existing review was generated against.
        reviewed_head_sha: String,
    },
}

impl CandidateStatus {
    /// Whether generating a review is the useful action for this candidate.
    pub fn wants_review(&self) -> bool {
        !matches!(self, Self::Reviewed { .. })
    }
}

/// A candidate paired with its relationship to local review history.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotatedCandidate {
    #[serde(flatten)]
    pub candidate: ReviewCandidate,
    pub status: CandidateStatus,
}

impl ReviewCandidate {
    /// Head SHA of the candidate, when the source carries one.
    pub fn head_sha(&self) -> Option<&str> {
        match &self.source {
            ReviewSource::GitHubPr { head_sha, .. } | ReviewSource::GitLabMr { head_sha, .. } => {
                head_sha.as_deref()
            }
            ReviewSource::DiffPaste { .. } => None,
        }
    }

    /// Whether this candidate and a review source refer to the same pull request.
    pub fn matches_source(&self, other: &ReviewSource) -> bool {
        match (&self.source, other) {
            (
                ReviewSource::GitHubPr {
                    owner: a_owner,
                    repo: a_repo,
                    number: a_number,
                    ..
                },
                ReviewSource::GitHubPr {
                    owner: b_owner,
                    repo: b_repo,
                    number: b_number,
                    ..
                },
            ) => {
                a_owner.eq_ignore_ascii_case(b_owner)
                    && a_repo.eq_ignore_ascii_case(b_repo)
                    && a_number == b_number
            }
            (
                ReviewSource::GitLabMr {
                    host: a_host,
                    project_path: a_path,
                    number: a_number,
                    ..
                },
                ReviewSource::GitLabMr {
                    host: b_host,
                    project_path: b_path,
                    number: b_number,
                    ..
                },
            ) => {
                a_host.eq_ignore_ascii_case(b_host)
                    && a_path.eq_ignore_ascii_case(b_path)
                    && a_number == b_number
            }
            _ => false,
        }
    }
}
