use crate::domain::{DiffRef, ReviewTask, RiskLevel, TaskStats, TaskStatus};
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
    diagram: Option<String>,
    #[serde(default)]
    sub_flow: Option<String>,
}

#[derive(Deserialize, Default)]
struct RawStats {
    #[serde(default)]
    risk: String,
    #[serde(default)]
    tags: Vec<String>,
}

// Support legacy diffs field for backward compatibility
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

// Extract files from diff_refs
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

    // Try to extract from params or arguments
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

pub(crate) fn parse_task(args: Value) -> Result<ReviewTask> {
    let normalized = normalize_single_task_payload(args)?;
    let task: SingleTaskPayload = serde_json::from_value(normalized)?;
    let stats = task.stats.unwrap_or_default();
    let risk = match stats.risk.to_uppercase().as_str() {
        "HIGH" => RiskLevel::High,
        "MEDIUM" | "MED" => RiskLevel::Medium,
        _ => RiskLevel::Low,
    };

    // Determine which field to use: diff_refs takes precedence, fall back to diffs
    let files = if !task.diff_refs.is_empty() {
        extract_files_from_diff_refs(&task.diff_refs)
    } else {
        extract_files_from_diffs_legacy(&task.diffs)
    };

    let (additions, deletions) = if !task.diff_refs.is_empty() {
        // If we have diff_refs, we'd need to calculate from the actual diff text using the diff index
        // For now, we'll fall back to the legacy calculation if diffs exist
        if !task.diffs.is_empty() {
            count_line_changes_legacy(&task.diffs)
        } else {
            (0, 0) // Placeholder values that will be recomputed later
        }
    } else {
        count_line_changes_legacy(&task.diffs)
    };

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
        insight: None,
        diagram: task.diagram,
        ai_generated: true,
        status: TaskStatus::Pending,
        sub_flow: task.sub_flow,
    })
}
