use crate::domain::{DiffRef, ReviewStatus, ReviewTask, RiskLevel, TaskStats};
use crate::infra::diagram::parse_json;
use anyhow::Result;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;

#[derive(Deserialize)]
struct SingleTaskPayload {
    id: String,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    stats: Option<RawStats>,
    #[serde(default)]
    diffs: Vec<String>,
    #[serde(default)]
    diff_refs: Vec<DiffRef>,
    #[serde(default)]
    diagram: Option<Value>,
    #[serde(default)]
    sub_flow: Option<String>,
    #[serde(default)]
    insight: Option<String>,
}

#[derive(Deserialize, Default)]
struct RawStats {
    #[serde(default)]
    risk: String,
    #[serde(default)]
    tags: Vec<String>,
}

/// Extract changed file paths from raw unified diff text.
///
/// This function provides backward compatibility for agents that emit
/// raw diff strings instead of structured `DiffRef` objects.
fn extract_files_from_diffs_legacy(diffs: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut files = Vec::new();
    for diff in diffs {
        for line in diff.lines() {
            if let Some(rest) = line.strip_prefix("diff --git ") {
                let mut parts = rest.split_whitespace();
                let a_path = parts.next().unwrap_or("");
                let b_path = parts.next().unwrap_or("");
                let candidate =
                    if !b_path.is_empty() && b_path != "b/dev/null" && b_path != "/dev/null" {
                        crate::infra::diff::normalize_task_path(b_path)
                    } else {
                        crate::infra::diff::normalize_task_path(a_path)
                    };
                if !candidate.is_empty()
                    && candidate != "dev/null"
                    && seen.insert(candidate.clone())
                {
                    files.push(candidate);
                }
            }
        }
    }
    files
}

/// Extract changed file paths from structured `DiffRef` objects.
fn extract_files_from_diff_refs(diff_refs: &[DiffRef]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut files = Vec::new();
    for diff_ref in diff_refs {
        if seen.insert(diff_ref.file.clone()) {
            files.push(diff_ref.file.clone());
        }
    }
    files
}

/// Calculate approximate line-level change statistics from unified diff text.
///
/// Used as a fallback mechanism when structured change counts are unavailable.
fn count_line_changes_legacy(diffs: &[String]) -> (u32, u32) {
    let mut additions = 0u32;
    let mut deletions = 0u32;
    for diff in diffs {
        for line in diff.lines() {
            if line.starts_with("+++") || line.starts_with("---") || line.starts_with("diff --git")
            {
                continue;
            }
            if line.starts_with('+') {
                additions += 1;
            } else if line.starts_with('-') {
                deletions += 1;
            }
        }
    }
    (additions, deletions)
}

fn normalize_single_task_payload(args: Value) -> Result<Value> {
    let mut current = args;

    if let Some(s) = current.as_str() {
        if let Ok(v) = serde_json::from_str::<Value>(s) {
            current = v;
        } else if s.contains("\"id\"") && (s.contains("\"title\"") || s.contains("\"description\""))
        {
            // Try to find the outermost object that looks like a task
            let mut brace_depth = 0;
            let mut start_idx = None;

            for (i, c) in s.char_indices() {
                match c {
                    '{' => {
                        if brace_depth == 0 {
                            start_idx = Some(i);
                        }
                        brace_depth += 1;
                    }
                    '}' => {
                        brace_depth -= 1;
                        if brace_depth == 0
                            && start_idx.is_some()
                            && let Some(start) = start_idx
                            && let Ok(v) = serde_json::from_str::<Value>(&s[start..=i])
                        {
                            // Check if it has required fields for a task
                            if v.get("id").is_some() && v.get("title").is_some() {
                                current = v;
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // If it already looks like a task with required fields, return it
    if current.get("id").is_some() && current.get("title").is_some() {
        return Ok(current);
    }

    // Attempt to extract task payload from standard protocol envelopes ('params' or 'arguments').
    if let Some(params) = current.get("params")
        && params.get("id").is_some()
        && params.get("title").is_some()
    {
        return Ok(params.clone());
    }

    if let Some(arguments) = current.get("arguments")
        && arguments.get("id").is_some()
        && arguments.get("title").is_some()
    {
        return Ok(arguments.clone());
    }

    Err(anyhow::anyhow!(
        "missing required fields `id` and `title` for task"
    ))
}

use std::sync::Arc;

pub(crate) fn parse_task(args: Value) -> Result<ReviewTask> {
    let normalized = normalize_single_task_payload(args)?;
    let task: SingleTaskPayload = serde_json::from_value(normalized)?;
    let stats = task.stats.unwrap_or_default();
    let risk = match stats.risk.to_uppercase().as_str() {
        "HIGH" => RiskLevel::High,
        "MEDIUM" | "MED" => RiskLevel::Medium,
        _ => RiskLevel::Low,
    };

    // Prefer structured file references (`diff_refs`) for path extraction;
    // fall back to legacy diff text parsing if necessary.
    let files = if !task.diff_refs.is_empty() {
        extract_files_from_diff_refs(&task.diff_refs)
    } else {
        extract_files_from_diffs_legacy(&task.diffs)
    };

    let (additions, deletions) = if !task.diff_refs.is_empty() {
        // When using `diff_refs`, precise statistics are recomputed during
        // the persistence phase using the global `DiffIndex`. If legacy
        // `diffs` are present, they are used as an initial heuristic.
        if !task.diffs.is_empty() {
            count_line_changes_legacy(&task.diffs)
        } else {
            (0, 0) // Placeholder; statistics will be recomputed during save_task.
        }
    } else {
        count_line_changes_legacy(&task.diffs)
    };

    let diagram = normalize_diagram_value(task.diagram)?;

    // Validate JSON by parsing, surface clear error
    if let Some(ref d) = diagram {
        parse_json(d).map_err(|e| anyhow::anyhow!(format!("Invalid diagram JSON: {e}")))?;
    }

    Ok(ReviewTask {
        id: task.id,
        run_id: String::new(), // set in persistence
        title: task.title,
        description: task.description,
        files,
        stats: TaskStats {
            additions,
            deletions,
            risk,
            tags: stats.tags,
        },
        diff_refs: task.diff_refs,
        insight: task.insight.map(Arc::from),
        diagram,
        ai_generated: true,
        status: ReviewStatus::Todo,
        sub_flow: task.sub_flow,
    })
}

fn normalize_diagram_value(value: Option<Value>) -> Result<Option<Arc<str>>> {
    let Some(value) = value else {
        return Ok(None);
    };

    if value.is_null() {
        return Ok(None);
    }

    if value.is_string() {
        anyhow::bail!("diagram must be a JSON object, not a string");
    }

    if !value.is_object() {
        anyhow::bail!("diagram must be a JSON object with type/data");
    }

    Ok(Some(Arc::from(value.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_files_from_diffs_legacy() {
        let diffs = vec![
            "diff --git a/src/main.rs b/src/main.rs\nindex ...".to_string(),
            "diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs".to_string(),
        ];
        let files = extract_files_from_diffs_legacy(&diffs);
        assert_eq!(files, vec!["src/main.rs", "src/lib.rs"]);
    }

    #[test]
    fn test_extract_files_legacy_deletion() {
        let diffs =
            vec!["diff --git a/deleted b/dev/null\n--- a/deleted\n+++ /dev/null".to_string()];
        let files = extract_files_from_diffs_legacy(&diffs);
        assert_eq!(files, vec!["deleted"]);
    }

    #[test]
    fn test_parse_task_basic() {
        let payload = json!({
            "id": "T1",
            "title": "Title",
            "description": "Desc",
            "diagram": {
                "type": "flow",
                "data": {
                    "direction": "LR",
                    "nodes": [
                        { "id": "a", "label": "A", "kind": "generic" },
                        { "id": "b", "label": "B", "kind": "generic" }
                    ],
                    "edges": [
                        { "from": "a", "to": "b", "label": "edge" }
                    ]
                }
            },
            "stats": { "risk": "HIGH", "tags": ["tag1"] },
            "diff_refs": [
                { "file": "test.rs", "hunks": [] }
            ]
        });
        let task = parse_task(payload).unwrap();
        assert_eq!(task.id, "T1");
        assert_eq!(task.stats.risk, RiskLevel::High);
        assert_eq!(task.files, vec!["test.rs"]);
    }

    #[test]
    fn test_parse_task_rejects_diagram_string() {
        let payload = json!({
            "id": "T2",
            "title": "Title2",
            "description": "Desc2",
            "diagram": "{\"type\":\"flow\",\"data\":{\"direction\":\"LR\",\"nodes\":[{\"id\":\"a\",\"label\":\"A\",\"kind\":\"generic\"}]}}",
            "stats": { "risk": "LOW", "tags": [] },
            "diff_refs": [
                { "file": "test.rs", "hunks": [] }
            ]
        });
        let err = parse_task(payload).unwrap_err();
        assert!(
            err.to_string().contains("diagram must be a JSON object"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_normalize_single_task_payload() {
        let raw = json!({
            "params": {
                "id": "T1",
                "title": "Title"
            }
        });
        let norm = normalize_single_task_payload(raw).unwrap();
        assert_eq!(norm.get("id").unwrap().as_str(), Some("T1"));
    }

    #[test]
    fn test_normalize_single_task_payload_string() {
        let raw = json!("{\"id\": \"T2\", \"title\": \"Title2\"}");
        let norm = normalize_single_task_payload(raw).unwrap();
        assert_eq!(norm.get("id").unwrap().as_str(), Some("T2"));
    }

    #[test]
    fn test_normalize_single_task_payload_nested_string() {
        let raw =
            json!("Here is your task: {\"id\": \"T3\", \"title\": \"Title3\"} hope you like it");
        let norm = normalize_single_task_payload(raw).unwrap();
        assert_eq!(norm.get("id").unwrap().as_str(), Some("T3"));
    }

    #[test]
    fn test_normalize_single_task_payload_arguments() {
        let raw = json!({
            "arguments": {
                "id": "T4",
                "title": "Title4"
            }
        });
        let norm = normalize_single_task_payload(raw).unwrap();
        assert_eq!(norm.get("id").unwrap().as_str(), Some("T4"));
    }

    #[test]
    fn test_normalize_single_task_payload_invalid() {
        let raw = json!({ "foo": "bar" });
        assert!(normalize_single_task_payload(raw).is_err());
    }

    #[test]
    fn test_count_line_changes_legacy() {
        let diffs = vec!["--- a/f\n+++ b/f\n+added\n-removed\n+added2".to_string()];
        let (add, del) = count_line_changes_legacy(&diffs);
        assert_eq!(add, 2);
        assert_eq!(del, 1);
    }
}
