//! Domain types for LaReview application
//! Defines the core data structures and business objects used throughout the application.

use serde::{Deserialize, Serialize};

pub mod repo;
pub use repo::*;

/// Unique identifier for a pull request
pub type ReviewId = String;

/// Unique identifier for a review generation run
pub type ReviewRunId = String;

/// Unique identifier for a review task
pub type TaskId = String;

/// Risk level associated with a review task, indicating the potential impact of the changes
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RiskLevel {
    /// Low risk changes with minimal impact
    #[default]
    Low,
    /// Medium risk changes that require attention
    Medium,
    /// High risk changes that require careful review
    High,
}

impl RiskLevel {
    pub fn rank(self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Medium => 1,
            Self::High => 2,
        }
    }
}

/// Source for a review.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReviewSource {
    /// Review is based purely on a pasted diff.
    DiffPaste {
        /// Hash of the unified diff used for the run that created this review.
        diff_hash: String,
    },
    /// Review is derived from a GitHub pull request fetched locally via `gh`.
    GitHubPr {
        owner: String,
        repo: String,
        number: u32,
        #[serde(default)]
        url: Option<String>,
        #[serde(default)]
        head_sha: Option<String>,
        #[serde(default)]
        base_sha: Option<String>,
    },
}

/// A review to be generated and tracked in the app.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    /// Unique identifier for the review.
    pub id: ReviewId,
    /// Agent-generated title (GitHub PR title may be used as initial title).
    pub title: String,
    /// Optional agent-generated summary of the review.
    pub summary: Option<String>,
    /// Where this review came from (diff paste or GitHub PR).
    pub source: ReviewSource,
    /// Latest run ID currently shown in the Review UI.
    pub active_run_id: Option<ReviewRunId>,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// A single generation run for a review (diff + agent output).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRun {
    /// Unique identifier for the run.
    pub id: ReviewRunId,
    /// Parent review.
    pub review_id: ReviewId,
    /// Agent id used for this run.
    pub agent_id: String,
    /// Raw user input reference (diff paste text or PR URL/ID).
    pub input_ref: String,
    /// Canonical unified diff used for this run.
    pub diff_text: String,
    /// Hash of `diff_text` for quick change checks/dedupe.
    pub diff_hash: String,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
}

/// A reference to a specific hunk within a file's diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HunkRef {
    /// The starting line number of the hunk in the old file.
    pub old_start: u32,
    /// The number of lines in the hunk from the old file.
    pub old_lines: u32,
    /// The starting line number of the hunk in the new file.
    pub new_start: u32,
    /// The number of lines in the hunk from the new file.
    pub new_lines: u32,
}

/// A reference to changed sections of a file, represented by a list of hunks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DiffRef {
    /// The path to the file that was changed.
    pub file: String,
    /// A list of specific hunks that are relevant to a task.
    pub hunks: Vec<HunkRef>,
}

/// Statistics for a review task
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskStats {
    /// Number of lines added in the changes
    pub additions: u32,
    /// Number of lines deleted in the changes
    pub deletions: u32,
    /// Risk level of the changes
    pub risk: RiskLevel,
    /// Tags describing the nature of the changes
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Status of a review task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum TaskStatus {
    /// Task has not been started yet
    #[default]
    #[serde(alias = "PENDING")]
    Pending,
    /// Task is currently being worked on
    #[serde(alias = "INPROGRESS")]
    #[serde(alias = "IN_PROGRESS")]
    #[serde(alias = "inprogress")]
    #[serde(alias = "in_progress")]
    InProgress,
    /// Task has been completed
    #[serde(alias = "REVIEWED")]
    #[serde(alias = "COMPLETED")]
    Done,
    /// Task has been reviewed but was determined to be ignorable
    #[serde(alias = "IGNORED")]
    Ignored,
}

impl TaskStatus {
    pub fn is_closed(self) -> bool {
        matches!(self, Self::Done | Self::Ignored)
    }
}

/// A review task spanning one or more files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewTask {
    /// Unique identifier for the task
    pub id: TaskId,
    /// ID of the review run this task belongs to
    pub run_id: ReviewRunId,
    /// Title of the review task
    pub title: String,
    /// Detailed description of the review task
    pub description: String,
    /// List of files affected by this task. Derived from `diff_refs`.
    pub files: Vec<String>,
    /// Statistical information about the changes
    pub stats: TaskStats,
    /// References to the specific diff hunks relevant to this task.
    #[serde(default)]
    pub diff_refs: Vec<DiffRef>,
    /// AI-generated insight about the task (optional)
    pub insight: Option<String>,
    /// Optional diagram describing the task context
    pub diagram: Option<String>,
    /// Flag indicating if the task was generated by AI
    #[serde(default)]
    pub ai_generated: bool,
    /// Current review status of the task
    #[serde(default)]
    pub status: TaskStatus,
    /// Optional sub-flow name this task belongs to for organizational purposes
    #[serde(default)]
    pub sub_flow: Option<String>,
}

/// Status of a plan entry
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    /// Plan entry has not been started yet
    #[default]
    #[serde(alias = "PENDING")]
    Pending,
    /// Plan entry is currently in progress
    #[serde(alias = "INPROGRESS")]
    #[serde(alias = "IN_PROGRESS")]
    #[serde(alias = "inprogress")]
    #[serde(alias = "in_progress")]
    InProgress,
    /// Plan entry has been completed
    #[serde(alias = "COMPLETED")]
    Completed,
}

/// Priority level of a plan entry
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PlanPriority {
    /// Low priority plan entry
    #[serde(alias = "LOW")]
    Low,
    /// Medium priority plan entry
    #[default]
    #[serde(alias = "MEDIUM")]
    Medium,
    /// High priority plan entry
    #[serde(alias = "HIGH")]
    High,
}

/// A single entry in a plan
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanEntry {
    /// Content/narrative description of the plan entry
    pub content: String,
    /// Priority level of the plan entry
    #[serde(default)]
    pub priority: PlanPriority,
    /// Current status of the plan entry
    #[serde(default)]
    pub status: PlanStatus,
    /// Optional metadata associated with the plan entry
    #[serde(default)]
    pub meta: Option<serde_json::Value>,
}

/// A plan containing multiple entries
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    /// List of plan entries
    pub entries: Vec<PlanEntry>,
    /// Optional metadata associated with the plan
    #[serde(default)]
    pub meta: Option<serde_json::Value>,
}

/// Status of a feedback thread
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThreadStatus {
    /// Work to do
    #[default]
    Todo,
    /// Work in progress
    Wip,
    /// Work completed
    Done,
    /// Declined or won't fix (can be reopened)
    Reject,
}

/// Impact/severity level for a thread
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThreadImpact {
    /// Optional/nit-level feedback
    #[default]
    Nitpick,
    /// Must address before merge
    Blocking,
    /// Nice to have before/after merge
    #[serde(alias = "nice-to-have")]
    NiceToHave,
}

/// Side of a diff line
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadSide {
    Old,
    New,
}

/// Optional anchor tying a thread to a file/line/hunk
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ThreadAnchor {
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub line_number: Option<u32>,
    #[serde(default)]
    pub side: Option<ThreadSide>,
    #[serde(default)]
    pub hunk_ref: Option<HunkRef>,
    #[serde(default)]
    pub head_sha: Option<String>,
}

/// Feedback thread spanning one or more comments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: String,
    pub review_id: ReviewId,
    #[serde(default)]
    pub task_id: Option<TaskId>,
    pub title: String,
    pub status: ThreadStatus,
    pub impact: ThreadImpact,
    #[serde(default)]
    pub anchor: Option<ThreadAnchor>,
    pub author: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Comment within a feedback thread
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub thread_id: String,
    pub author: String,
    pub body: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Mapping to an external provider thread (e.g., GitHub)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadLink {
    pub id: String,
    pub thread_id: String,
    pub provider: String,
    pub provider_thread_id: String,
    pub provider_root_comment_id: String,
    pub last_synced_at: String,
}
