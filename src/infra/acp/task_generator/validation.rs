use crate::domain::ReviewTask;
use anyhow::Result;
use std::collections::HashSet;

pub(super) fn validate_tasks_payload(
    tasks: &[ReviewTask],
    raw_payload: Option<&serde_json::Value>,
    diff_text: &str,
) -> Result<Vec<String>> {
    if tasks.len() < 2 || tasks.len() > 7 {
        anyhow::bail!("return_tasks must provide 2-7 tasks, got {}", tasks.len());
    }

    if let Some(raw) = raw_payload {
        let tasks_arr = raw
            .get("tasks")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("return_tasks payload missing tasks array"))?;
        for (idx, t) in tasks_arr.iter().enumerate() {
            let risk_str = t
                .get("stats")
                .and_then(|s| s.get("risk"))
                .and_then(|r| r.as_str())
                .map(|s| s.to_uppercase());
            match risk_str.as_deref() {
                Some("LOW") | Some("MEDIUM") | Some("HIGH") | Some("MED") => {}
                Some(other) => anyhow::bail!("Task {idx} has invalid stats.risk '{other}'"),
                None => anyhow::bail!("Task {idx} missing stats.risk"),
            }
        }
    }

    let changed_files = crate::infra::diff::extract_changed_files(diff_text);
    let mentioned_files: HashSet<String> = tasks
        .iter()
        .flat_map(|task| task.files.iter())
        .map(|f| crate::infra::diff::normalize_task_path(f))
        .collect();

    let missing: Vec<String> = changed_files
        .difference(&mentioned_files)
        .cloned()
        .collect();
    if !missing.is_empty() {
        anyhow::bail!(
            "Tasks do not cover all changed files. Missing: {}",
            missing.join(", ")
        );
    }

    // Optional: ensure task diffs are substrings of the provided diff.
    let mut warnings = Vec::new();
    let diff_norm = diff_text.replace("\r\n", "\n");
    for task in tasks {
        if task.diffs.iter().any(|d| !diff_norm.contains(d)) {
            warnings.push(format!(
                "Task {} includes diffs not found in provided <diff>",
                task.id
            ));
        }
    }

    Ok(warnings)
}
