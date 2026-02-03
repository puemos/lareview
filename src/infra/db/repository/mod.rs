//! Repository implementations for data access in LaReview.
//!
//! Provides database operations for reviews, runs, tasks, feedback, and comments.

mod comment;
mod feedback;
mod feedback_link;
mod issue_check;
mod learned_patterns;
mod merge_confidence;
mod rejections;
mod repo;
mod review;
mod review_run;
mod rule;
mod task;

pub use comment::CommentRepository;
pub use feedback::FeedbackRepository;
pub use feedback_link::FeedbackLinkRepository;
pub use issue_check::IssueCheckRepository;
pub use learned_patterns::{LearnedPatternRepository, LearningStateRepository};
pub use merge_confidence::MergeConfidenceRepository;
pub use rejections::{
    AgentRejectionStats, FeedbackRejection, FeedbackRejectionRepository, RuleRejectionStats,
};
pub use repo::RepoRepository;
pub use review::ReviewRepository;
pub use review_run::ReviewRunRepository;
pub use rule::ReviewRuleRepository;
pub use task::TaskRepository;

use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub(super) type DbConn = Arc<Mutex<Connection>>;

/// Marker trait for repository types.
///
/// This trait documents that a type follows the common repository pattern
/// of being constructed with a `DbConn`.
pub trait Repository {}

#[cfg(test)]
mod tests;
