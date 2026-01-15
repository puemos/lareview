//! Snapshot session domain types.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents an active snapshot session for a PR review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotSession {
    /// Unique session identifier.
    pub id: String,
    /// ID of the linked repository.
    pub repo_id: String,
    /// Path to the snapshot directory.
    pub snapshot_path: PathBuf,
    /// Commit SHA the snapshot was created from.
    pub commit_sha: String,
    /// When the session was created.
    pub created_at: String,
}
