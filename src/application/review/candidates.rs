//! Relating review candidates to the reviews already held locally.

use crate::domain::{AnnotatedCandidate, CandidateStatus, Review, ReviewCandidate};

/// Annotates each candidate with whether it has been reviewed, and whether that
/// review is still current.
///
/// A candidate is stale when a review exists for it but the pull request's head
/// commit has moved on since that review was generated. Candidates whose head
/// SHA is unknown are treated as reviewed rather than stale — claiming staleness
/// without evidence would be worse than staying quiet.
pub fn annotate_candidates(
    candidates: Vec<ReviewCandidate>,
    reviews: &[Review],
) -> Vec<AnnotatedCandidate> {
    candidates
        .into_iter()
        .map(|candidate| {
            let status = status_for(&candidate, reviews);
            AnnotatedCandidate { candidate, status }
        })
        .collect()
}

fn status_for(candidate: &ReviewCandidate, reviews: &[Review]) -> CandidateStatus {
    // Most recent review for this pull request wins. `created_at` is RFC3339,
    // which orders correctly as a string.
    let existing = reviews
        .iter()
        .filter(|review| candidate.matches_source(&review.source))
        .max_by(|a, b| a.created_at.cmp(&b.created_at));

    let Some(review) = existing else {
        return CandidateStatus::New;
    };

    let reviewed_head = match &review.source {
        crate::domain::ReviewSource::GitHubPr { head_sha, .. }
        | crate::domain::ReviewSource::GitLabMr { head_sha, .. } => head_sha.as_deref(),
        crate::domain::ReviewSource::DiffPaste { .. } => None,
    };

    match (candidate.head_sha(), reviewed_head) {
        (Some(current), Some(reviewed)) if current != reviewed => CandidateStatus::Stale {
            review_id: review.id.clone(),
            reviewed_head_sha: reviewed.to_string(),
        },
        _ => CandidateStatus::Reviewed {
            review_id: review.id.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{CandidateReason, ReviewSource, ReviewStatus};
    use chrono::Utc;

    fn candidate(number: u32, head_sha: Option<&str>) -> ReviewCandidate {
        ReviewCandidate {
            provider_id: "github".to_string(),
            source: ReviewSource::GitHubPr {
                owner: "acme".to_string(),
                repo: "widget".to_string(),
                number,
                url: None,
                head_sha: head_sha.map(str::to_string),
                base_sha: None,
            },
            title: format!("PR {number}"),
            url: format!("https://github.com/acme/widget/pull/{number}"),
            author: "someone".to_string(),
            repo: "acme/widget".to_string(),
            number,
            updated_at: Utc::now(),
            reason: CandidateReason::ReviewRequested,
            is_draft: false,
        }
    }

    /// `created_at` is RFC3339; the timestamp only needs to order predictably.
    fn review(id: &str, number: u32, head_sha: Option<&str>, created_at: &str) -> Review {
        Review {
            id: id.to_string(),
            title: format!("Review {id}"),
            summary: None,
            source: ReviewSource::GitHubPr {
                owner: "acme".to_string(),
                repo: "widget".to_string(),
                number,
                url: None,
                head_sha: head_sha.map(str::to_string),
                base_sha: None,
            },
            active_run_id: None,
            status: ReviewStatus::default(),
            created_at: created_at.to_string(),
            updated_at: created_at.to_string(),
        }
    }

    #[test]
    fn unreviewed_candidate_is_new() {
        let annotated = annotate_candidates(vec![candidate(1, Some("abc"))], &[]);
        assert_eq!(annotated[0].status, CandidateStatus::New);
    }

    #[test]
    fn candidate_reviewed_at_current_head_is_reviewed() {
        let reviews = vec![review("r1", 1, Some("abc"), "2026-01-01T00:00:00Z")];
        let annotated = annotate_candidates(vec![candidate(1, Some("abc"))], &reviews);
        assert_eq!(
            annotated[0].status,
            CandidateStatus::Reviewed {
                review_id: "r1".to_string()
            }
        );
    }

    #[test]
    fn candidate_whose_head_moved_on_is_stale() {
        let reviews = vec![review("r1", 1, Some("old"), "2026-01-01T00:00:00Z")];
        let annotated = annotate_candidates(vec![candidate(1, Some("new"))], &reviews);
        assert_eq!(
            annotated[0].status,
            CandidateStatus::Stale {
                review_id: "r1".to_string(),
                reviewed_head_sha: "old".to_string()
            }
        );
    }

    #[test]
    fn unknown_head_sha_does_not_claim_staleness() {
        let reviews = vec![review("r1", 1, Some("old"), "2026-01-01T00:00:00Z")];
        let annotated = annotate_candidates(vec![candidate(1, None)], &reviews);
        assert_eq!(
            annotated[0].status,
            CandidateStatus::Reviewed {
                review_id: "r1".to_string()
            }
        );
    }

    #[test]
    fn reviews_for_other_pull_requests_are_ignored() {
        let reviews = vec![review("r1", 99, Some("abc"), "2026-01-01T00:00:00Z")];
        let annotated = annotate_candidates(vec![candidate(1, Some("abc"))], &reviews);
        assert_eq!(annotated[0].status, CandidateStatus::New);
    }

    #[test]
    fn most_recent_review_wins() {
        let reviews = vec![
            review("old", 1, Some("sha1"), "2026-01-01T00:00:00Z"),
            review("new", 1, Some("sha2"), "2026-02-01T00:00:00Z"),
        ];
        let annotated = annotate_candidates(vec![candidate(1, Some("sha2"))], &reviews);
        assert_eq!(
            annotated[0].status,
            CandidateStatus::Reviewed {
                review_id: "new".to_string()
            }
        );
    }

    #[test]
    fn wants_review_is_false_only_when_current() {
        assert!(CandidateStatus::New.wants_review());
        assert!(
            CandidateStatus::Stale {
                review_id: "r".into(),
                reviewed_head_sha: "s".into()
            }
            .wants_review()
        );
        assert!(
            !CandidateStatus::Reviewed {
                review_id: "r".into()
            }
            .wants_review()
        );
    }
}
