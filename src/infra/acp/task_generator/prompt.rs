use crate::domain::{DefaultIssueCategory, ResolvedRule};
use crate::infra::acp::task_mcp_server::RunContext;
use crate::prompts;
use agent_client_protocol::{ClientCapabilities, FileSystemCapability, Meta};
use anyhow::Context;
use serde_json::json;
use std::path::PathBuf;

/// Rule item structure for the template
#[derive(serde::Serialize)]
struct RuleItem {
    category: String,
    display_name: String,
    text: String,
    glob: Option<String>,
    scope: String,
    has_matches: bool,
    matched_files: Vec<String>,
    rule_id: Option<String>,
}

pub(super) fn build_prompt(
    run: &RunContext,
    repo_root: Option<&PathBuf>,
    rules: &[ResolvedRule],
) -> anyhow::Result<String> {
    let has_repo_access = repo_root.is_some();
    let source_json = serde_json::to_string(&run.source).unwrap_or_default();

    // Generate unified manifest for agents (replaces all previous manifest formats)
    let unified_manifest = match crate::infra::diff::index::DiffIndex::new(&run.diff_text) {
        Ok(index) => index.generate_unified_manifest(),
        Err(_) => String::new(),
    };

    // Convert all rules to rule items for the template
    let rule_items: Vec<RuleItem> = rules
        .iter()
        .map(|rule| RuleItem {
            category: rule.category.clone().unwrap_or_else(|| rule.id.clone()),
            display_name: rule
                .category
                .as_ref()
                .map(|c| format_category_name(c))
                .unwrap_or_else(|| "Custom Rule".to_string()),
            text: rule.text.clone(),
            glob: rule.glob.clone(),
            scope: rule.scope.to_string(),
            has_matches: rule.has_matches,
            matched_files: rule.matched_files.clone(),
            rule_id: Some(rule.id.clone()),
        })
        .collect();

    // Get default issue categories (could be filtered by user settings in the future)
    let default_categories = DefaultIssueCategory::defaults();

    prompts::render(
        "generate_tasks",
        &json!({
            "review_id": run.review_id,
            "source_json": source_json,
            "initial_title": run.initial_title,
            "diff": run.diff_text,
            "unified_manifest": unified_manifest,
            "has_repo_access": has_repo_access,
            "repo_root": repo_root.map(|p| p.display().to_string()),
            "repo_access_note": if has_repo_access { "read-only" } else { "none" },
            // All rules are treated equally - verified by AI
            "has_rules": !rule_items.is_empty(),
            "rules": rule_items,
            // Default categories (built-in)
            "has_default_categories": !default_categories.is_empty(),
            "default_categories": default_categories,
        }),
    )
    .context("failed to render generate_tasks prompt")
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

pub(super) fn build_client_capabilities(has_repo_access: bool) -> ClientCapabilities {
    let fs_cap = if has_repo_access {
        FileSystemCapability::new()
            .read_text_file(true)
            .write_text_file(false)
    } else {
        FileSystemCapability::new()
            .read_text_file(false)
            .write_text_file(false)
    };

    ClientCapabilities::new()
        .fs(fs_cap)
        .terminal(false)
        .meta(Meta::from_iter([
            (
                "lareview-return-task".into(),
                serde_json::json!({
                    "type": "extension",
                    "method": "lareview/return_task",
                    "description": "Submit a single review task back to the client as structured data",
                    "params": {
                        "id": "string",
                        "title": "string",
                        "description": "string",
                        "stats": {
                            "additions": "number",
                            "deletions": "number",
                            "risk": "LOW|MEDIUM|HIGH",
                            "tags": ["string"]
                        },
                        "insight": "string",
                        "diagram": "flow LR x[label=X kind=generic] y[label=Y kind=generic] x --> y[label=rel]",
                        "sub_flow": "string (optional grouping)",
                        "hunk_ids": ["string (e.g., 'src/main.rs#H1')"]
                    }
                }),
            ),
            (
                "lareview-finalize-review".into(),
                serde_json::json!({
                    "type": "extension",
                    "method": "lareview/finalize_review",
                    "description": "Submit review title and summary to finalize the review",
                    "params": {
                        "title": "string",
                        "summary": "string"
                    }
                }),
            ),
            (
                "lareview-return-plans".into(),
                serde_json::json!({
                    "type": "extension",
                    "method": "lareview/return_plans",
                    "description": "Submit review plans back to the client as structured data",
                    "params": {
                        "plans": [{
                            "entries": [{
                                "content": "string",
                                "priority": "LOW|MEDIUM|HIGH",
                                "status": "PENDING|IN_PROGRESS|COMPLETED",
                                "meta": "object"
                            }],
                            "meta": "object"
                        }]
                    }
                }),
            ),
        ]))
}
