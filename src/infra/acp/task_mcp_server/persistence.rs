//! Persistence functions for MCP server tools

use super::config::ServerConfig;
use super::task_ingest::{load_run_context, open_database};
use crate::domain::{
    CheckStatus, Comment, Confidence, Feedback, FeedbackAnchor, FeedbackImpact, FeedbackSide,
    IssueCheck, IssueFinding, ReviewStatus,
};
use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::Value;
use std::str::FromStr;

/// Save an issue check report from the agent
pub fn save_issue_check(config: &ServerConfig, args: Value) -> Result<String> {
    let ctx = load_run_context(config);
    let db = open_database(config)?;
    let issue_check_repo = db.issue_check_repo();

    // Parse required fields
    let category = args
        .get("category")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing required field: category"))?
        .to_string();

    let status_str = args
        .get("status")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing required field: status"))?;

    let status = CheckStatus::from_str(status_str)
        .map_err(|e| anyhow::anyhow!("invalid status '{}': {}", status_str, e))?;

    let confidence_str = args
        .get("confidence")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing required field: confidence"))?;

    let confidence = Confidence::from_str(confidence_str)
        .map_err(|e| anyhow::anyhow!("invalid confidence '{}': {}", confidence_str, e))?;

    // Parse optional fields
    let rule_id = args
        .get("rule_id")
        .and_then(|v| v.as_str())
        .map(normalize_rule_id);

    let display_name = args
        .get("display_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format_category_name(&category));

    let summary = args
        .get("summary")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Generate check ID
    let check_id = format!("check-{}", uuid::Uuid::new_v4());
    let now = Utc::now().to_rfc3339();

    // Create and save the issue check
    let check = IssueCheck {
        id: check_id.clone(),
        run_id: ctx.run_id.clone(),
        rule_id,
        category,
        display_name,
        status,
        confidence,
        summary,
        created_at: now.clone(),
    };

    issue_check_repo
        .save(&check)
        .with_context(|| format!("save issue check {}", check_id))?;

    // Parse and save findings if present, also create corresponding Feedback entries with initial comments
    let feedback_repo = db.feedback_repo();
    let comment_repo = db.comment_repo();
    if let Some(findings) = args.get("findings").and_then(|v| v.as_array()) {
        for (idx, finding_value) in findings.iter().enumerate() {
            let finding = parse_finding(finding_value, &check_id, idx, &now)?;
            issue_check_repo
                .save_finding(&finding)
                .with_context(|| format!("save finding for check {}", check_id))?;

            // Create a Feedback entry for this finding so it can be pushed to VCS
            let feedback = create_feedback_from_finding(
                &finding,
                &ctx.review_id,
                check.rule_id.as_deref(),
                &now,
            );
            feedback_repo
                .save(&feedback)
                .with_context(|| format!("save feedback for finding {}", finding.id))?;

            // Create an initial comment with the finding's description and evidence
            let comment = create_comment_from_finding(&finding, &feedback.id, &now);
            comment_repo
                .save(&comment)
                .with_context(|| format!("save initial comment for finding {}", finding.id))?;
        }
    }

    Ok(check_id)
}

/// Create a Comment with the finding's description and evidence
fn create_comment_from_finding(finding: &IssueFinding, feedback_id: &str, now: &str) -> Comment {
    let body = format!(
        "{}\n\n**Evidence:** {}",
        finding.description, finding.evidence
    );

    Comment {
        id: format!("comment-{}", uuid::Uuid::new_v4()),
        feedback_id: feedback_id.to_string(),
        author: "agent:system".to_string(),
        body,
        parent_id: None,
        created_at: now.to_string(),
        updated_at: now.to_string(),
    }
}

/// Create a Feedback entry from an IssueFinding
fn create_feedback_from_finding(
    finding: &IssueFinding,
    review_id: &str,
    rule_id: Option<&str>,
    now: &str,
) -> Feedback {
    let feedback_id = format!("feedback-{}", uuid::Uuid::new_v4());

    let anchor = if finding.file_path.is_some() || finding.line_number.is_some() {
        Some(FeedbackAnchor {
            file_path: finding.file_path.clone(),
            line_number: finding.line_number,
            side: Some(FeedbackSide::New),
            hunk_ref: None,
            head_sha: None,
        })
    } else {
        None
    };

    Feedback {
        id: feedback_id,
        review_id: review_id.to_string(),
        task_id: None,
        rule_id: rule_id.map(|s| s.to_string()),
        finding_id: Some(finding.id.clone()),
        title: finding.title.clone(),
        status: ReviewStatus::Todo,
        impact: finding.impact,
        anchor,
        author: "agent".to_string(),
        created_at: now.to_string(),
        updated_at: now.to_string(),
    }
}

/// Parse a finding from JSON value
fn parse_finding(value: &Value, check_id: &str, idx: usize, now: &str) -> Result<IssueFinding> {
    let title = value
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("finding {} missing required field: title", idx))?
        .to_string();

    let description = value
        .get("description")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("finding {} missing required field: description", idx))?
        .to_string();

    let evidence = value
        .get("evidence")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("finding {} missing required field: evidence", idx))?
        .to_string();

    let impact_str = value
        .get("impact")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("finding {} missing required field: impact", idx))?;

    let impact = FeedbackImpact::from_str(impact_str).unwrap_or(FeedbackImpact::Nitpick);

    let file_path = value
        .get("file_path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let line_number = value
        .get("line_number")
        .and_then(|v| v.as_u64())
        .map(|n| n as u32);

    let finding_id = format!("finding-{}", uuid::Uuid::new_v4());

    Ok(IssueFinding {
        id: finding_id,
        check_id: check_id.to_string(),
        title,
        description,
        evidence,
        file_path,
        line_number,
        impact,
        created_at: now.to_string(),
    })
}

/// Normalize rule_id by stripping scope prefix if present (e.g., "global|rule-123" -> "rule-123")
fn normalize_rule_id(raw: &str) -> String {
    if let Some(pos) = raw.find('|') {
        raw[pos + 1..].to_string()
    } else {
        raw.to_string()
    }
}

/// Format a category ID into a display name
fn format_category_name(category: &str) -> String {
    category
        .split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_rule_id() {
        assert_eq!(normalize_rule_id("rule-123"), "rule-123");
        assert_eq!(normalize_rule_id("global|rule-123"), "rule-123");
        assert_eq!(normalize_rule_id("repo|my-rule"), "my-rule");
    }

    #[test]
    fn test_format_category_name() {
        assert_eq!(format_category_name("security"), "Security");
        assert_eq!(format_category_name("breaking-changes"), "Breaking Changes");
        assert_eq!(format_category_name("error-handling"), "Error Handling");
    }
}
