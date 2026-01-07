use crate::domain::{DiffRef, HunkRef, ReviewTask};
use crate::infra::diff::index::DiffIndex;
use anyhow::{Result, anyhow};
use serde_json::Value;
use std::collections::HashSet;

pub(super) fn validate_tasks_payload(
    tasks: &[ReviewTask],
    raw_payload: Option<&serde_json::Value>,
    diff_text: &str,
) -> Result<Vec<String>> {
    for (idx, task) in tasks.iter().enumerate() {
        let risk_str = match task.stats.risk {
            crate::domain::RiskLevel::Low => "LOW",
            crate::domain::RiskLevel::Medium => "MEDIUM",
            crate::domain::RiskLevel::High => "HIGH",
        };
        match risk_str {
            "LOW" | "MEDIUM" | "HIGH" => {}
            other => anyhow::bail!("Task {idx} has invalid stats.risk '{other}'"),
        }

        let has_diagram = task
            .diagram
            .as_ref()
            .is_some_and(|diagram| !diagram.trim().is_empty());
        if !has_diagram {
            anyhow::bail!(
                "Task {} is missing a diagram JSON block. Every task must include a diagram.",
                task.id
            );
        }
    }

    let diff_index = DiffIndex::new(diff_text)?;
    let mut warnings = Vec::new();

    if let Some(payload) = raw_payload
        && let Some(tasks_array) = payload.get("tasks").and_then(|t| t.as_array())
    {
        for (task_idx, task_val) in tasks_array.iter().enumerate() {
            if let Some(hunk_ids_val) = task_val.get("hunk_ids")
                && let Some(hunk_ids_array) = hunk_ids_val.as_array()
            {
                let task_id_str = task_val
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("task_{}", task_idx));

                warnings.push(format!(
                    "Task {} uses new hunk_ids format. Converting to diff_refs automatically.",
                    task_id_str
                ));

                let diff_refs =
                    convert_hunk_ids_to_diff_refs(&diff_index, hunk_ids_array, &task_id_str)?;
                warnings.push(format!(
                    "Task {}: converted {} hunk_ids to {} diff_refs",
                    task_id_str,
                    hunk_ids_array.len(),
                    diff_refs.len()
                ));
            }
        }
    }

    for task in tasks {
        for diff_ref in &task.diff_refs {
            if diff_ref.hunks.is_empty() {
                if let Err(err) = diff_index.validate_file_exists(diff_ref.file.as_str()) {
                    if let Some(diff_index_err) =
                        err.downcast_ref::<crate::infra::diff::index::DiffIndexError>()
                    {
                        anyhow::bail!(
                            "Task {} references file {} that does not exist in diff. Error: {}. Use file paths from the hunk manifest.",
                            task.id,
                            diff_ref.file,
                            diff_index_err
                        );
                    } else {
                        anyhow::bail!(
                            "Task {} references file {} that does not exist in diff. Use file paths from the hunk manifest.",
                            task.id,
                            diff_ref.file
                        );
                    }
                }
                continue;
            }

            for hunk_ref in &diff_ref.hunks {
                if let Err(err) = diff_index.validate_hunk_exists(diff_ref.file.as_str(), hunk_ref)
                {
                    if let Some(diff_index_err) =
                        err.downcast_ref::<crate::infra::diff::index::DiffIndexError>()
                    {
                        let nearest = diff_index_err.nearest();
                        let nearest_new_start = nearest.map(|(_, n)| n).unwrap_or(0);
                        anyhow::bail!(
                            "Task {} references invalid hunk in {} at old_start={}, new_start={}.\n\n\
                             **To fix:**\n\
                             - Copy hunk IDs from the manifest (e.g., 'src/auth.rs#H3')\n\
                             - Or use the new 'hunk_ids' field instead of diff_refs\n\n\
                             **Nearest hunk:** new_start={}",
                            task.id,
                            diff_ref.file,
                            hunk_ref.old_start,
                            hunk_ref.new_start,
                            nearest_new_start
                        );
                    } else {
                        anyhow::bail!(
                            "Task {} references invalid hunk in {} at old_start={}, new_start={}. Copy coordinates from the hunk manifest.",
                            task.id,
                            diff_ref.file,
                            hunk_ref.old_start,
                            hunk_ref.new_start
                        );
                    }
                }
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

    Ok(warnings)
}

fn convert_hunk_ids_to_diff_refs(
    diff_index: &DiffIndex,
    hunk_ids: &[Value],
    task_id: &str,
) -> Result<Vec<DiffRef>> {
    let mut hunks_by_file: std::collections::HashMap<String, Vec<HunkRef>> =
        std::collections::HashMap::new();

    for hunk_id_val in hunk_ids {
        let hunk_id = hunk_id_val.as_str().ok_or_else(|| {
            anyhow!(
                "Task {}: hunk_ids must contain strings, got: {}",
                task_id,
                hunk_id_val
            )
        })?;

        let coords = diff_index.get_hunk_coords(hunk_id).ok_or_else(|| {
            let suggestions = list_hunk_suggestions(diff_index, hunk_id);
            anyhow!(
                "Task {} references invalid hunk_id '{}'.\n\n{}\n\n\
                     **Valid format:** 'path/to/file#H1' (e.g., 'src/auth.rs#H3')",
                task_id,
                hunk_id,
                suggestions
            )
        })?;

        let (file_path, _) = diff_index
            .parse_hunk_id(hunk_id)
            .ok_or_else(|| anyhow!("Invalid hunk_id format: {}", hunk_id))?;

        hunks_by_file.entry(file_path).or_default().push(coords);
    }

    let mut diff_refs: Vec<DiffRef> = hunks_by_file
        .into_iter()
        .map(|(file, hunks)| DiffRef { file, hunks })
        .collect();

    diff_refs.sort_by(|a, b| a.file.cmp(&b.file));
    Ok(diff_refs)
}

fn list_hunk_suggestions(diff_index: &DiffIndex, attempted: &str) -> String {
    let (partial_path, _) = attempted.rsplit_once('#').unwrap_or((attempted, ""));

    let mut suggestions = Vec::new();

    for file_path in diff_index.get_all_file_paths() {
        if file_path.contains(partial_path) || partial_path.is_empty() {
            let hunk_ids = diff_index.get_hunk_ids_for_file(&file_path);
            for hunk_id in hunk_ids.iter().take(5) {
                suggestions.push(format!("  - {}", hunk_id));
            }
            if hunk_ids.len() > 5 {
                suggestions.push(format!("  ... and {} more", hunk_ids.len() - 5));
            }
            break;
        }
    }

    if suggestions.is_empty() {
        "No matching files found. Use the exact file path from the hunk manifest.".to_string()
    } else {
        format!("Did you mean one of these?\n{}", suggestions.join("\n"))
    }
}
