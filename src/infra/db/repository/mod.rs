//! Repository implementations for data access in LaReview.
//!
//! Provides database operations for reviews, runs, tasks, feedback, and comments.

mod comment;
mod feedback;
mod repo;
mod review;
mod review_run;
mod task;

pub use comment::CommentRepository;
pub use feedback::FeedbackRepository;
pub use repo::RepoRepository;
pub use review::ReviewRepository;
pub use review_run::ReviewRunRepository;
pub use task::TaskRepository;

use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub(super) type DbConn = Arc<Mutex<Connection>>;

#[cfg(test)]
mod tests;
