//! Domain types for LaReview application
//! Defines the core data structures and business objects used throughout the application.

pub mod error;
pub mod feedback;
pub mod repo;
pub mod review;
pub mod task;

pub use error::*;
pub use feedback::*;
pub use repo::*;
pub use review::*;
pub use task::*;

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
