//! Learned pattern domain model for AI-powered learning from rejected feedback.

use serde::{Deserialize, Serialize};

/// A learned pattern that guides future reviews to avoid unhelpful feedback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPattern {
    pub id: String,
    /// The pattern text, e.g., "Don't flag unwrap() in test files"
    pub pattern_text: String,
    /// Category like "testing", "performance", "style"
    pub category: Option<String>,
    /// File extension filter, e.g., "rs", "ts", or None for all files
    pub file_extension: Option<String>,
    /// Number of rejections that contributed to this pattern
    pub source_count: i32,
    /// Whether the user manually edited this pattern
    pub is_edited: bool,
    /// Whether the pattern is currently active
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for creating or updating a learned pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPatternInput {
    pub pattern_text: String,
    pub category: Option<String>,
    pub file_extension: Option<String>,
    pub enabled: Option<bool>,
}

/// Status of the learning system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningStatus {
    /// Number of rejections waiting to be processed
    pub pending_rejections: i64,
    /// When compaction last ran
    pub last_compaction_at: Option<String>,
    /// Total number of learned patterns
    pub pattern_count: i64,
    /// Number of enabled patterns
    pub enabled_pattern_count: i64,
    /// Threshold for triggering automatic compaction
    pub compaction_threshold: i64,
}

impl Default for LearningStatus {
    fn default() -> Self {
        Self {
            pending_rejections: 0,
            last_compaction_at: None,
            pattern_count: 0,
            enabled_pattern_count: 0,
            compaction_threshold: 10,
        }
    }
}

/// Result of a learning compaction run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningCompactionResult {
    /// Number of rejections processed
    pub rejections_processed: usize,
    /// Number of new patterns created
    pub patterns_created: usize,
    /// Number of existing patterns updated
    pub patterns_updated: usize,
    /// Any errors that occurred
    pub errors: Vec<String>,
}
