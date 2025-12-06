#![allow(dead_code)]
//! Domain types for LaReview

use serde::{Deserialize, Serialize};

/// Unique identifier for a pull request
pub type PullRequestId = String;

/// Unique identifier for a review task
pub type TaskId = String;

/// Risk level for a task
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RiskLevel {
    #[default]
    Low,
    Medium,
    High,
}

/// A pull request to be reviewed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub id: PullRequestId,
    pub title: String,
    pub description: Option<String>,
    pub repo: String,
    pub author: String,
    pub branch: String,
    pub created_at: String,
}

/// Statistics for a review task
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskStats {
    pub additions: u32,
    pub deletions: u32,
    pub risk: RiskLevel,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// A patch hunk for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    pub file: String,
    pub hunk: String,
}

/// A review task spanning one or more files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewTask {
    pub id: TaskId,
    pub title: String,
    pub description: String,
    pub files: Vec<String>,
    pub stats: TaskStats,
    #[serde(default)]
    pub patches: Vec<Patch>,
    pub insight: Option<String>,
    pub diagram: Option<String>,
    #[serde(default)]
    pub ai_generated: bool,
}

/// A note attached to a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub task_id: TaskId,
    pub body: String,
    pub updated_at: String,
}

/// Parsed file diff from git
#[derive(Debug, Clone)]
pub struct ParsedFileDiff {
    pub file_path: String,
    pub patch: String,
    pub additions: u32,
    pub deletions: u32,
}
