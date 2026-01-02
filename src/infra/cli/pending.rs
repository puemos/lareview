//! Pending review serialization for CLI-to-GUI handoff.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const PENDING_FILENAME: &str = "pending_review.json";

/// Data passed from CLI to GUI
#[derive(Debug, Serialize, Deserialize)]
pub struct PendingReview {
    /// The diff to review
    pub diff: String,

    /// Local repo root (if detected)
    pub repo_root: Option<PathBuf>,

    /// ACP agent to use
    pub agent: Option<String>,

    /// Auto-generate tasks on load
    pub auto_generate: bool,

    /// Source of the diff (for display)
    pub source: String,

    /// Timestamp for cleanup
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Get the pending review file path
fn get_pending_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().context("Could not determine config directory")?;

    let lareview_dir = config_dir.join("lareview");

    if !lareview_dir.exists() {
        fs::create_dir_all(&lareview_dir).context("Failed to create LaReview config directory")?;
    }

    Ok(lareview_dir.join(PENDING_FILENAME))
}

/// Save pending review to temp file
pub fn save_pending_review(review: &PendingReview) -> Result<PathBuf> {
    let pending_path = get_pending_path()?;
    let json =
        serde_json::to_string_pretty(review).context("Failed to serialize pending review")?;

    fs::write(&pending_path, json).context("Failed to write pending review file")?;

    Ok(pending_path)
}

/// Load pending review from temp file
pub fn load_pending_review() -> Result<Option<PendingReview>> {
    let pending_path = get_pending_path()?;

    if !pending_path.exists() {
        return Ok(None);
    }

    let json = fs::read_to_string(&pending_path).context("Failed to read pending review file")?;

    let review: PendingReview =
        serde_json::from_str(&json).context("Failed to parse pending review file")?;

    fs::remove_file(&pending_path).ok();

    Ok(Some(review))
}

/// Check if a pending review file exists
pub fn has_pending_review() -> bool {
    get_pending_path().map(|p| p.exists()).unwrap_or(false)
}

/// Clean up any stale pending review files
pub fn cleanup_stale_reviews() -> Result<()> {
    if let Ok(pending_path) = get_pending_path()
        && pending_path.exists()
    {
        fs::remove_file(&pending_path)?;
    }
    Ok(())
}
