use crate::domain::{ReviewRule, RuleScope};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRuleInput {
    pub scope: String,
    pub repo_id: Option<String>,
    pub glob: Option<String>,
    pub category: Option<String>,
    pub text: String,
    pub enabled: bool,
}

#[tauri::command]
pub fn get_review_rules(state: State<'_, AppState>) -> Result<Vec<ReviewRule>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.rule_repo().list_all().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_review_rule(
    state: State<'_, AppState>,
    input: ReviewRuleInput,
) -> Result<ReviewRule, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let rule_id = Uuid::new_v4().to_string();
    let rule = build_review_rule(rule_id, now.clone(), now, input)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.rule_repo().save(&rule).map_err(|e| e.to_string())?;
    Ok(rule)
}

#[tauri::command]
pub fn update_review_rule(
    state: State<'_, AppState>,
    id: String,
    input: ReviewRuleInput,
) -> Result<ReviewRule, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let existing = db
        .rule_repo()
        .find_by_id(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Review rule not found".to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let rule = build_review_rule(id, existing.created_at, now, input)?;
    db.rule_repo().save(&rule).map_err(|e| e.to_string())?;
    Ok(rule)
}

#[tauri::command]
pub fn delete_review_rule(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.rule_repo().delete(&id).map_err(|e| e.to_string())?;
    Ok(())
}

fn build_review_rule(
    id: String,
    created_at: String,
    updated_at: String,
    input: ReviewRuleInput,
) -> Result<ReviewRule, String> {
    let scope = RuleScope::from_str(&input.scope.to_lowercase()).map_err(|e| e.to_string())?;
    let repo_id = input.repo_id.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    let glob = input.glob.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    let category = input.category.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    let text = input.text.trim().to_string();
    if text.is_empty() {
        return Err("Rule text cannot be empty".to_string());
    }

    match scope {
        RuleScope::Global => {
            if repo_id.is_some() {
                return Err("Global rules cannot target a repository".to_string());
            }
        }
        RuleScope::Repo => {
            if repo_id.is_none() {
                return Err("Repository rules require a repo_id".to_string());
            }
        }
    }

    Ok(ReviewRule {
        id,
        scope,
        repo_id,
        glob,
        category,
        text,
        enabled: input.enabled,
        created_at,
        updated_at,
    })
}

// ============================================================================
// Rule Library Commands
// ============================================================================

/// Get all rules from the library
#[tauri::command]
pub fn get_rule_library() -> Vec<LibraryRule> {
    LibraryRule::all()
}

/// Get library rules filtered by category
#[tauri::command]
pub fn get_rule_library_by_category(category: String) -> Vec<LibraryRule> {
    let category = match category.to_lowercase().as_str() {
        "security" => LibraryCategory::Security,
        "code_quality" | "codequality" => LibraryCategory::CodeQuality,
        "testing" => LibraryCategory::Testing,
        "documentation" => LibraryCategory::Documentation,
        "performance" => LibraryCategory::Performance,
        "api_design" | "apidesign" => LibraryCategory::ApiDesign,
        "language_specific" | "languagespecific" => LibraryCategory::LanguageSpecific,
        "framework_specific" | "frameworkspecific" => LibraryCategory::FrameworkSpecific,
        _ => return Vec::new(),
    };
    LibraryRule::by_category(category)
}

/// Add a rule from the library to the user's rules
#[tauri::command]
pub fn add_rule_from_library(
    state: State<'_, AppState>,
    library_rule_id: String,
    scope: String,
    repo_id: Option<String>,
) -> Result<ReviewRule, String> {
    // Find the library rule
    let library_rule = LibraryRule::all()
        .into_iter()
        .find(|r| r.id == library_rule_id)
        .ok_or_else(|| format!("Library rule not found: {}", library_rule_id))?;

    let now = chrono::Utc::now().to_rfc3339();
    let rule_id = Uuid::new_v4().to_string();

    let input = ReviewRuleInput {
        scope,
        repo_id,
        glob: library_rule.glob.clone(),
        category: library_rule.category.clone(),
        text: library_rule.text.clone(),
        enabled: true,
    };

    let rule = build_review_rule(rule_id, now.clone(), now, input)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.rule_repo().save(&rule).map_err(|e| e.to_string())?;
    Ok(rule)
}

use crate::domain::{LibraryCategory, LibraryRule};

/// Get default issue categories
#[tauri::command]
pub fn get_default_issue_categories() -> Vec<DefaultIssueCategory> {
    DefaultIssueCategory::defaults()
}

use crate::domain::DefaultIssueCategory;

// ============================================================================
// Rule Effectiveness / Analytics Commands
// ============================================================================

/// Statistics about rejection rates for a rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleRejectionStatsResponse {
    pub rule_id: String,
    pub total_feedback: i64,
    pub rejected_count: i64,
    pub rejection_rate: f64,
}

/// Get rejection statistics for all rules (for rule effectiveness dashboard)
#[tauri::command]
pub fn get_rule_rejection_stats(
    state: State<'_, AppState>,
) -> Result<Vec<RuleRejectionStatsResponse>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let stats = db
        .rejection_repo()
        .get_rule_stats()
        .map_err(|e| e.to_string())?;

    Ok(stats
        .into_iter()
        .map(|s| RuleRejectionStatsResponse {
            rule_id: s.rule_id,
            total_feedback: s.total_feedback,
            rejected_count: s.rejected_count,
            rejection_rate: s.rejection_rate,
        })
        .collect())
}
