use super::review::ReviewId;
use super::task::{HunkRef, ReviewStatus, TaskId};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

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

impl fmt::Display for FeedbackSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Old => write!(f, "old"),
            Self::New => write!(f, "new"),
        }
    }
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
    /// Optional rule ID that inspired this feedback
    #[serde(default)]
    pub rule_id: Option<String>,
    /// Links to an IssueFinding if created from rule check
    #[serde(default)]
    pub finding_id: Option<String>,
    /// Category ID from issue check (e.g., "test-coverage", "security")
    #[serde(default)]
    pub category: Option<String>,
    /// Brief title/summary of the feedback
    pub title: String,
    /// Current status of the feedback
    pub status: ReviewStatus,
    /// Impact/severity of the feedback
    pub impact: FeedbackImpact,
    /// Confidence score (0.0-1.0) indicating how certain the AI is about this feedback
    #[serde(default = "default_confidence")]
    pub confidence: f64,
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

/// Default confidence score (1.0 = high confidence)
fn default_confidence() -> f64 {
    1.0
}
