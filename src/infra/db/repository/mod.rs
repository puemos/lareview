//! Repository implementations for data access in LaReview.
//!
//! Provides database operations for reviews, runs, tasks, feedback, and comments.

mod comment;
mod feedback;
mod feedback_link;
mod repo;
mod review;
mod review_run;
mod task;

pub use comment::CommentRepository;
pub use feedback::FeedbackRepository;
pub use feedback_link::FeedbackLinkRepository;
pub use repo::RepoRepository;
pub use review::ReviewRepository;
pub use review_run::ReviewRunRepository;
pub use task::TaskRepository;

use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub(super) type DbConn = Arc<Mutex<Connection>>;

/// Marker trait for repository types.
///
/// This trait documents that a type follows the common repository pattern
/// of being constructed with a `DbConn`. It does not provide a default
/// implementation to avoid breaking existing code that calls `.new()` directly.
///
/// # Example
///
/// ```rust
/// use crate::infra::DbConn;
///
/// trait Repository {}
///
/// struct MyRepository {
///     conn: DbConn,
/// }
///
/// impl Repository for MyRepository {}
/// ```
pub trait Repository {}

#[cfg(test)]
mod tests;
