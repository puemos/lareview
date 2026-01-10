//! Repository implementations for data access in LaReview.
//!
//! Provides database operations for reviews, runs, tasks, feedback, and comments.

mod comment;
mod feedback;
mod feedback_link;
mod repo;
mod review;
mod review_run;
mod rule;
mod task;

pub use comment::CommentRepository;
pub use feedback::FeedbackRepository;
pub use feedback_link::FeedbackLinkRepository;
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
