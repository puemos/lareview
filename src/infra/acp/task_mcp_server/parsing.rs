use crate::domain::{ReviewTask, RiskLevel, TaskStats, TaskStatus};
use anyhow::Result;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashSet;

#[derive(Deserialize)]
struct TasksPayload {
    tasks: Vec<RawTask>,
}

#[derive(Deserialize)]
struct RawTask {
    id: String,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    stats: Option<RawStats>,
    #[serde(default)]
    diffs: Vec<String>,
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

fn extract_files_from_diffs(diffs: &[String]) -> Vec<String> {
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

fn count_line_changes(diffs: &[String]) -> (u32, u32) {
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

fn normalize_tasks_payload(args: Value) -> Result<Value> {
    let mut current = args;

    if let Some(s) = current.as_str() {
        if let Ok(v) = serde_json::from_str::<Value>(s) {
            current = v;
        } else if s.contains("\"tasks\"")
            && let Some(tasks_pos) = s.find("\"tasks\"")
        {
            let start = s[..tasks_pos].rfind('{');
            let end = s.rfind('}');
            if let (Some(start), Some(end)) = (start, end)
                && let Ok(v) = serde_json::from_str::<Value>(&s[start..=end])
            {
                current = v;
            }
        }
    }

    if current.get("tasks").is_some() {
        return Ok(current);
    }

    if let Some(params) = current.get("params")
        && params.get("tasks").is_some()
    {
        return Ok(params.clone());
    }

    if let Some(arguments) = current.get("arguments")
        && arguments.get("tasks").is_some()
    {
        return Ok(arguments.clone());
    }

    if current.is_array() {
        return Ok(json!({ "tasks": current }));
    }

    Err(anyhow::anyhow!("missing field `tasks`"))
}

pub(crate) fn parse_tasks(args: Value) -> Result<Vec<ReviewTask>> {
    let normalized = normalize_tasks_payload(args)?;
    let payload: TasksPayload = serde_json::from_value(normalized)?;
    let tasks = payload
        .tasks
        .into_iter()
        .map(|task| {
            let stats = task.stats.unwrap_or_default();
            let risk = match stats.risk.to_uppercase().as_str() {
                "HIGH" => RiskLevel::High,
                "MEDIUM" | "MED" => RiskLevel::Medium,
                _ => RiskLevel::Low,
            };

            let computed_files = extract_files_from_diffs(&task.diffs);
            let (additions, deletions) = count_line_changes(&task.diffs);

            ReviewTask {
                id: task.id,
                run_id: String::new(), // set in persistence
                title: task.title,
                description: task.description,
                files: computed_files,
                stats: TaskStats {
                    additions,
                    deletions,
                    risk,
                    tags: stats.tags,
                },
                diffs: task.diffs,
                insight: None,
                diagram: task.diagram,
                ai_generated: true,
                status: TaskStatus::Pending,
                sub_flow: task.sub_flow,
            }
        })
        .collect();

    Ok(tasks)
}
