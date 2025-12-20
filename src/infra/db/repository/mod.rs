//! Repository implementations for data access in LaReview.
//!
//! Provides database operations for reviews, runs, tasks, threads, and comments.

mod comment;
mod repo;
mod review;
mod review_run;
mod task;
mod thread;

pub use comment::CommentRepository;
pub use repo::RepoRepository;
pub use review::ReviewRepository;
pub use review_run::ReviewRunRepository;
pub use task::TaskRepository;
pub use thread::ThreadRepository;

use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub(super) type DbConn = Arc<Mutex<Connection>>;

#[cfg(test)]
mod tests;
