//! Repository implementations for data access in LaReview.
//!
//! Provides database operations for pull requests, tasks, and notes.

mod note;
mod pull_request;
mod task;

pub use note::NoteRepository;
pub use pull_request::PullRequestRepository;
pub use task::TaskRepository;

use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub(super) type DbConn = Arc<Mutex<Connection>>;

#[cfg(test)]
mod tests;
