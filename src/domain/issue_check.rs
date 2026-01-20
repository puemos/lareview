//! Domain types for issue checklist verification
//!
//! Issue checks track whether specific categories of issues (security, breaking changes, etc.)
//! were verified during a review run.

use super::feedback::FeedbackImpact;
use super::review::ReviewRunId;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Unique identifier for an issue check
pub type IssueCheckId = String;

/// Unique identifier for a finding within an issue check
pub type FindingId = String;

/// Status of an issue check category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    /// Issues were detected in this category
    Found,
    /// Checked thoroughly, no issues found
    #[default]
    NotFound,
    /// Category doesn't apply to this review (e.g., no DB changes for data integrity)
    NotApplicable,
    /// Could not fully check (e.g., missing context)
    Skipped,
}

impl fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Found => write!(f, "found"),
            Self::NotFound => write!(f, "not_found"),
            Self::NotApplicable => write!(f, "not_applicable"),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

impl FromStr for CheckStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "found" => Ok(Self::Found),
            "not_found" | "notfound" => Ok(Self::NotFound),
            "not_applicable" | "notapplicable" | "n/a" | "na" => Ok(Self::NotApplicable),
            "skipped" => Ok(Self::Skipped),
            other => Err(format!("invalid check status: {other}")),
        }
    }
}

/// Confidence level for an issue check
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// High confidence in the assessment
    #[default]
    High,
    /// Medium confidence - some uncertainty
    Medium,
    /// Low confidence - significant uncertainty
    Low,
}

impl fmt::Display for Confidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::High => write!(f, "high"),
            Self::Medium => write!(f, "medium"),
            Self::Low => write!(f, "low"),
        }
    }
}

impl FromStr for Confidence {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "high" => Ok(Self::High),
            "medium" | "med" => Ok(Self::Medium),
            "low" => Ok(Self::Low),
            other => Err(format!("invalid confidence level: {other}")),
        }
    }
}

/// An issue check representing verification of a specific category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueCheck {
    /// Unique identifier for this check
    pub id: IssueCheckId,
    /// Review run this check belongs to
    pub run_id: ReviewRunId,
    /// Optional rule ID if this check is from a custom checklist rule
    #[serde(default)]
    pub rule_id: Option<String>,
    /// Category name (e.g., "security", "breaking-changes", "performance")
    pub category: String,
    /// Display name for the category
    pub display_name: String,
    /// Status of the check
    pub status: CheckStatus,
    /// Confidence level in the assessment
    pub confidence: Confidence,
    /// Brief summary of findings or why N/A
    #[serde(default)]
    pub summary: Option<String>,
    /// Creation timestamp in RFC3339 format
    pub created_at: String,
}

/// A specific finding within an issue check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueFinding {
    /// Unique identifier for this finding
    pub id: FindingId,
    /// Parent issue check ID
    pub check_id: IssueCheckId,
    /// Brief title of the finding
    pub title: String,
    /// Detailed description of the issue
    pub description: String,
    /// Evidence supporting the finding (code snippet, reasoning)
    pub evidence: String,
    /// File path where the issue was found (optional)
    #[serde(default)]
    pub file_path: Option<String>,
    /// Line number in the file (optional)
    #[serde(default)]
    pub line_number: Option<u32>,
    /// Impact/severity of this finding
    pub impact: FeedbackImpact,
    /// Creation timestamp in RFC3339 format
    pub created_at: String,
}

/// Default issue categories that are built-in
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultIssueCategory {
    /// Category identifier (e.g., "security")
    pub id: String,
    /// Display name (e.g., "Security")
    pub name: String,
    /// Description of what this category checks for
    pub description: String,
    /// Examples of issues in this category
    pub examples: Vec<String>,
    /// Whether this category is enabled by default
    pub enabled_by_default: bool,
}

impl DefaultIssueCategory {
    /// Returns all default issue categories
    pub fn defaults() -> Vec<Self> {
        vec![
            Self {
                id: "security".to_string(),
                name: "Security".to_string(),
                description: "Check for authentication issues, injection vulnerabilities, secrets exposure, and data protection problems.".to_string(),
                examples: vec![
                    "Missing auth check on endpoint".to_string(),
                    "SQL injection vulnerability".to_string(),
                    "Hardcoded API key or secret".to_string(),
                    "Sensitive data logged or exposed".to_string(),
                ],
                enabled_by_default: true,
            },
            Self {
                id: "breaking-changes".to_string(),
                name: "Breaking Changes".to_string(),
                description: "Identify API changes, removed exports, signature changes, and other breaking modifications.".to_string(),
                examples: vec![
                    "Removed public function or export".to_string(),
                    "Changed function signature".to_string(),
                    "Modified return type".to_string(),
                    "Renamed field without alias".to_string(),
                ],
                enabled_by_default: true,
            },
            Self {
                id: "error-handling".to_string(),
                name: "Error Handling".to_string(),
                description: "Verify proper error handling, validation, and graceful failure modes.".to_string(),
                examples: vec![
                    "Uncaught exception in async code".to_string(),
                    "Missing input validation".to_string(),
                    "Unsafe unwrap on user input".to_string(),
                    "Silent error swallowing".to_string(),
                ],
                enabled_by_default: true,
            },
            Self {
                id: "data-integrity".to_string(),
                name: "Data Integrity".to_string(),
                description: "Check for database issues, data loss risks, and consistency problems.".to_string(),
                examples: vec![
                    "Missing database transaction".to_string(),
                    "Race condition on shared state".to_string(),
                    "Destructive migration without backup".to_string(),
                    "Data truncation risk".to_string(),
                ],
                enabled_by_default: true,
            },
            Self {
                id: "performance".to_string(),
                name: "Performance".to_string(),
                description: "Identify performance issues like N+1 queries, memory leaks, and inefficient algorithms.".to_string(),
                examples: vec![
                    "N+1 query pattern".to_string(),
                    "Unbounded collection growth".to_string(),
                    "Missing pagination".to_string(),
                    "Expensive operation in loop".to_string(),
                ],
                enabled_by_default: true,
            },
            Self {
                id: "test-coverage".to_string(),
                name: "Test Coverage".to_string(),
                description: "Ensure critical paths and edge cases have appropriate test coverage.".to_string(),
                examples: vec![
                    "New feature without tests".to_string(),
                    "Edge case not covered".to_string(),
                    "Error path untested".to_string(),
                    "Security-critical code untested".to_string(),
                ],
                enabled_by_default: false,
            },
        ]
    }
}
