//! Domain types for merge confidence scoring.
//!
//! Provides a PR-level "Merge Confidence Score" (1.0-5.0 scale) where the AI review agent
//! directly submits their confidence evaluation with bullet point explanations.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Complete merge confidence assessment for a review run.
///
/// The agent evaluates the PR and provides:
/// - A score (1.0-5.0) indicating merge readiness
/// - Bullet point reasons explaining the score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeConfidence {
    /// The confidence score (1.0-5.0) from agent
    pub score: f32,
    /// Bullet point explanations for the score
    pub reasons: Vec<String>,
    /// When this assessment was computed (RFC3339)
    pub computed_at: String,
}

impl MergeConfidence {
    /// Creates a new merge confidence assessment.
    pub fn new(score: f32, reasons: Vec<String>) -> Self {
        Self {
            score: score.clamp(1.0, 5.0),
            reasons,
            computed_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Returns the score rounded to nearest integer (1-5).
    pub fn score_rounded(&self) -> u8 {
        self.score.round().clamp(1.0, 5.0) as u8
    }

    /// Returns a human-readable label for the score.
    pub fn label(&self) -> &'static str {
        match self.score_rounded() {
            5 => "Very Confident",
            4 => "Confident",
            3 => "Moderate",
            2 => "Low",
            _ => "Very Low",
        }
    }

    /// Returns a recommendation message for the score.
    pub fn recommendation(&self) -> &'static str {
        match self.score_rounded() {
            5 => "Ship it - low risk, well-tested",
            4 => "Looks good - minor concerns only",
            3 => "Needs attention - review carefully",
            2 => "Proceed with caution - significant concerns",
            _ => "Don't merge yet - blocking issues identified",
        }
    }
}

impl Default for MergeConfidence {
    fn default() -> Self {
        Self {
            score: 3.0, // Moderate by default
            reasons: Vec::new(),
            computed_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

impl fmt::Display for MergeConfidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1}/5", self.score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_clamping() {
        let mc = MergeConfidence::new(0.5, vec![]);
        assert_eq!(mc.score, 1.0);

        let mc = MergeConfidence::new(6.0, vec![]);
        assert_eq!(mc.score, 5.0);

        let mc = MergeConfidence::new(3.5, vec![]);
        assert_eq!(mc.score, 3.5);
    }

    #[test]
    fn test_score_rounded() {
        let mc = MergeConfidence::new(4.6, vec![]);
        assert_eq!(mc.score_rounded(), 5);

        let mc = MergeConfidence::new(4.4, vec![]);
        assert_eq!(mc.score_rounded(), 4);

        let mc = MergeConfidence::new(2.5, vec![]);
        assert_eq!(mc.score_rounded(), 3); // 2.5 rounds to 3
    }

    #[test]
    fn test_score_labels() {
        assert_eq!(MergeConfidence::new(5.0, vec![]).label(), "Very Confident");
        assert_eq!(MergeConfidence::new(4.0, vec![]).label(), "Confident");
        assert_eq!(MergeConfidence::new(3.0, vec![]).label(), "Moderate");
        assert_eq!(MergeConfidence::new(2.0, vec![]).label(), "Low");
        assert_eq!(MergeConfidence::new(1.0, vec![]).label(), "Very Low");
    }

    #[test]
    fn test_recommendations() {
        assert_eq!(
            MergeConfidence::new(5.0, vec![]).recommendation(),
            "Ship it - low risk, well-tested"
        );
        assert_eq!(
            MergeConfidence::new(4.0, vec![]).recommendation(),
            "Looks good - minor concerns only"
        );
        assert_eq!(
            MergeConfidence::new(3.0, vec![]).recommendation(),
            "Needs attention - review carefully"
        );
        assert_eq!(
            MergeConfidence::new(2.0, vec![]).recommendation(),
            "Proceed with caution - significant concerns"
        );
        assert_eq!(
            MergeConfidence::new(1.0, vec![]).recommendation(),
            "Don't merge yet - blocking issues identified"
        );
    }

    #[test]
    fn test_with_reasons() {
        let reasons = vec![
            "✓ Comprehensive test coverage".to_string(),
            "⚠ Missing error handling in parser".to_string(),
        ];
        let mc = MergeConfidence::new(4.0, reasons.clone());
        assert_eq!(mc.reasons.len(), 2);
        assert_eq!(mc.reasons, reasons);
    }

    #[test]
    fn test_display() {
        let mc = MergeConfidence::new(4.5, vec![]);
        assert_eq!(format!("{}", mc), "4.5/5");
    }
}
