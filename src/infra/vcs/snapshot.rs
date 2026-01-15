//! Git snapshot operations for temporary commit checkouts.
//!
//! Provides a manager for creating and removing lightweight snapshots of a commit.
//! Used to give agents access to PR code at a specific commit without
//! affecting the user's working directory.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Base directory name for LaReview snapshots in temp directory.
const SNAPSHOT_DIR_PREFIX: &str = "lareview-snapshots";

/// Manages git snapshots for a repository.
#[derive(Debug, Clone)]
pub struct SnapshotManager {
    /// Path to the main repository.
    repo_path: PathBuf,
}

impl SnapshotManager {
    /// Create a new snapshot manager for the given repository.
    pub fn new(repo_path: impl Into<PathBuf>) -> Self {
        Self {
            repo_path: repo_path.into(),
        }
    }

    /// Get the base directory for snapshots in the temp directory.
    fn snapshot_base_dir() -> PathBuf {
        std::env::temp_dir().join(SNAPSHOT_DIR_PREFIX)
    }

    /// Create a snapshot of the specified commit.
    ///
    /// Returns the path to the created snapshot directory.
    pub async fn create(&self, session_id: &str, commit_sha: &str) -> Result<PathBuf> {
        let base_dir = Self::snapshot_base_dir();

        // Ensure the base directory exists
        std::fs::create_dir_all(&base_dir)
            .with_context(|| format!("create snapshot base dir: {}", base_dir.display()))?;

        let snapshot_path = base_dir.join(session_id);
        if snapshot_path.exists() {
            std::fs::remove_dir_all(&snapshot_path).with_context(|| {
                format!("remove existing snapshot dir: {}", snapshot_path.display())
            })?;
        }
        std::fs::create_dir_all(&snapshot_path)
            .with_context(|| format!("create snapshot dir: {}", snapshot_path.display()))?;

        let index_file = tempfile::NamedTempFile::new().context("create temp index file")?;
        let index_path = index_file.path().to_string_lossy().to_string();

        let read_tree = Command::new("git")
            .args(["-C", &self.repo_path.to_string_lossy()])
            .args(["read-tree", commit_sha])
            .env("GIT_INDEX_FILE", &index_path)
            .output()
            .await
            .context("run git read-tree")?;

        if !read_tree.status.success() {
            let stderr = String::from_utf8_lossy(&read_tree.stderr);
            return Err(anyhow::anyhow!("git read-tree failed: {}", stderr));
        }

        let prefix = format!("{}/", snapshot_path.to_string_lossy());
        let checkout = Command::new("git")
            .args(["-C", &self.repo_path.to_string_lossy()])
            .args(["checkout-index", "-a", "--prefix", &prefix])
            .env("GIT_INDEX_FILE", &index_path)
            .output()
            .await
            .context("run git checkout-index")?;

        if !checkout.status.success() {
            let stderr = String::from_utf8_lossy(&checkout.stderr);
            return Err(anyhow::anyhow!("git checkout-index failed: {}", stderr));
        }

        Ok(snapshot_path)
    }

    /// Remove a snapshot and clean up.
    pub async fn remove(&self, snapshot_path: &Path) -> Result<()> {
        if snapshot_path.exists() {
            std::fs::remove_dir_all(snapshot_path)
                .with_context(|| format!("remove snapshot dir: {}", snapshot_path.display()))?;
            log::info!("Cleaned up snapshot at {}", snapshot_path.display());
        }
        Ok(())
    }
}
