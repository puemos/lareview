//! Domain types for LaReview application
//! Defines the core data structures and business objects used throughout the application.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

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

impl fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "LOW"),
            Self::Medium => write!(f, "MEDIUM"),
            Self::High => write!(f, "HIGH"),
        }
    }
}

impl FromStr for RiskLevel {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "LOW" => Ok(Self::Low),
            "MEDIUM" => Ok(Self::Medium),
            "HIGH" => Ok(Self::High),
            _ => Err(format!("Unknown risk level: {}", s)),
        }
    }
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
        /// GitHub owner (organization or user)
        owner: String,
        /// GitHub repository name
        repo: String,
        /// Pull request number
        number: u32,
        /// Optional canonical URL for the PR
        #[serde(default)]
        url: Option<String>,
        /// Topmost commit SHA of the PR
        #[serde(default)]
        head_sha: Option<String>,
        /// Base commit SHA of the target branch
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

use std::sync::Arc;

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
    pub diff_text: Arc<str>,
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

/// Status of a review item (task or feedback)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum ReviewStatus {
    /// Work to do
    #[default]
    #[serde(alias = "PENDING")]
    #[serde(alias = "TODO")]
    Todo,
    /// Work in progress
    #[serde(alias = "INPROGRESS")]
    #[serde(alias = "IN_PROGRESS")]
    #[serde(alias = "inprogress")]
    #[serde(alias = "in_progress")]
    #[serde(alias = "WIP")]
    #[serde(alias = "wip")]
    InProgress,
    /// Work completed
    #[serde(alias = "REVIEWED")]
    #[serde(alias = "COMPLETED")]
    #[serde(alias = "DONE")]
    Done,
    /// Work ignored or rejected
    #[serde(alias = "IGNORED")]
    #[serde(alias = "REJECT")]
    #[serde(alias = "REJECTED")]
    Ignored,
}

impl fmt::Display for ReviewStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Todo => write!(f, "todo"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Done => write!(f, "done"),
            Self::Ignored => write!(f, "ignored"),
        }
    }
}

impl FromStr for ReviewStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "PENDING" | "TODO" => Ok(Self::Todo),
            "IN_PROGRESS" | "INPROGRESS" | "WIP" => Ok(Self::InProgress),
            "DONE" | "REVIEWED" | "COMPLETED" => Ok(Self::Done),
            "IGNORED" | "REJECT" | "REJECTED" => Ok(Self::Ignored),
            _ => Ok(Self::Todo),
        }
    }
}

impl ReviewStatus {
    pub fn is_closed(self) -> bool {
        matches!(self, Self::Done | Self::Ignored)
    }

    pub fn rank(self) -> u8 {
        match self {
            Self::Todo => 0,
            Self::InProgress => 1,
            Self::Ignored => 2,
            Self::Done => 3,
        }
    }
}

/// A review task spanning one or more files
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    pub insight: Option<Arc<str>>,
    /// Optional diagram describing the task context
    pub diagram: Option<Arc<str>>,
    /// Flag indicating if the task was generated by AI
    #[serde(default)]
    pub ai_generated: bool,
    /// Current review status of the task
    #[serde(default)]
    pub status: ReviewStatus,
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

impl From<agent_client_protocol::Plan> for Plan {
    fn from(p: agent_client_protocol::Plan) -> Self {
        Self {
            entries: p.entries.into_iter().map(PlanEntry::from).collect(),
            meta: p.meta.map(serde_json::Value::Object),
        }
    }
}

impl From<agent_client_protocol::PlanEntry> for PlanEntry {
    fn from(e: agent_client_protocol::PlanEntry) -> Self {
        Self {
            content: e.content,
            priority: match e.priority {
                agent_client_protocol::PlanEntryPriority::Low => PlanPriority::Low,
                agent_client_protocol::PlanEntryPriority::Medium => PlanPriority::Medium,
                agent_client_protocol::PlanEntryPriority::High => PlanPriority::High,
                _ => PlanPriority::Medium,
            },
            status: match e.status {
                agent_client_protocol::PlanEntryStatus::Pending => PlanStatus::Pending,
                agent_client_protocol::PlanEntryStatus::InProgress => PlanStatus::InProgress,
                agent_client_protocol::PlanEntryStatus::Completed => PlanStatus::Completed,
                _ => PlanStatus::Pending,
            },
            meta: e.meta.map(serde_json::Value::Object),
        }
    }
}

/// Impact/severity level for feedback
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackImpact {
    /// Optional/nit-level feedback
    #[default]
    Nitpick,
    /// Must address before merge
    Blocking,
    /// Nice to have before/after merge
    #[serde(alias = "nice-to-have")]
    NiceToHave,
}

impl fmt::Display for FeedbackImpact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nitpick => write!(f, "nitpick"),
            Self::Blocking => write!(f, "blocking"),
            Self::NiceToHave => write!(f, "nice_to_have"),
        }
    }
}

impl FromStr for FeedbackImpact {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "nitpick" => Ok(Self::Nitpick),
            "blocking" => Ok(Self::Blocking),
            "nice_to_have" | "nice-to-have" => Ok(Self::NiceToHave),
            _ => Ok(Self::Nitpick),
        }
    }
}

/// Side of a diff line
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackSide {
    /// Line changed from the old version
    Old,
    /// Line changed or added in the new version
    New,
}

/// Optional anchor tying feedback to a file/line/hunk
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FeedbackAnchor {
    /// Relative path to the file
    #[serde(default)]
    pub file_path: Option<String>,
    /// Line number in the file
    #[serde(default)]
    pub line_number: Option<u32>,
    /// Which side of the diff the comment is on
    #[serde(default)]
    pub side: Option<FeedbackSide>,
    /// Specific hunk reference (optional)
    #[serde(default)]
    pub hunk_ref: Option<HunkRef>,
    /// Commit SHA (optional)
    #[serde(default)]
    pub head_sha: Option<String>,
}

/// Feedback entry spanning one or more comments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback {
    /// Unique identifier for the feedback
    pub id: String,
    /// Parent review ID
    pub review_id: ReviewId,
    /// Linked task ID (optional)
    #[serde(default)]
    pub task_id: Option<TaskId>,
    /// Brief title/summary of the feedback
    pub title: String,
    /// Current status of the feedback
    pub status: ReviewStatus,
    /// Impact/severity of the feedback
    pub impact: FeedbackImpact,
    /// Location of the feedback in the code
    #[serde(default)]
    pub anchor: Option<FeedbackAnchor>,
    /// Author identifier (agent or user)
    pub author: String,
    /// Creation timestamp
    pub created_at: String,
    /// Last update timestamp
    pub updated_at: String,
}

/// Comment within a feedback entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    /// Unique identifier for the comment
    pub id: String,
    /// Parent feedback ID
    pub feedback_id: String,
    /// Author identifier
    pub author: String,
    /// Body text of the comment (markdown)
    pub body: String,
    /// Parent comment ID (for nested replies)
    #[serde(default)]
    pub parent_id: Option<String>,
    /// Creation timestamp
    pub created_at: String,
    /// Last update timestamp
    pub updated_at: String,
}

/// Mapping to an external provider feedback (e.g., GitHub)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackLink {
    /// Unique identifier for the link
    pub id: String,
    /// Local feedback ID
    pub feedback_id: String,
    /// Provider name (e.g., "github")
    pub provider: String,
    /// ID on the provider side
    pub provider_feedback_id: String,
    /// Root comment ID on the provider side
    pub provider_root_comment_id: String,
    /// Last sync timestamp
    pub last_synced_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_risk_level_display_parse() {
        assert_eq!(RiskLevel::Low.to_string(), "LOW");
        assert_eq!(RiskLevel::from_str("HIGH").unwrap(), RiskLevel::High);
        assert!(RiskLevel::from_str("invalid").is_err());
    }

    #[test]
    fn test_review_status_display_parse() {
        assert_eq!(ReviewStatus::Todo.to_string(), "todo");
        assert_eq!(ReviewStatus::from_str("DONE").unwrap(), ReviewStatus::Done);
        assert_eq!(
            ReviewStatus::from_str("WIP").unwrap(),
            ReviewStatus::InProgress
        );
    }

    #[test]
    fn test_feedback_impact_display_parse() {
        assert_eq!(FeedbackImpact::Nitpick.to_string(), "nitpick");
        assert_eq!(
            FeedbackImpact::from_str("BLOCKING").unwrap(),
            FeedbackImpact::Blocking
        );
    }
}
