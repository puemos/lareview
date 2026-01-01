use crate::domain::ReviewTask;
use crate::infra::diff::index::DiffIndex;
use anyhow::Result;
use std::collections::HashSet;

pub(super) fn validate_tasks_payload(
    tasks: &[ReviewTask],
    _raw_payload: Option<&serde_json::Value>,
    diff_text: &str,
) -> Result<Vec<String>> {
    // Validate risk levels from the actual ReviewTask objects
    for (idx, task) in tasks.iter().enumerate() {
        let risk_str = match task.stats.risk {
            crate::domain::RiskLevel::Low => "LOW",
            crate::domain::RiskLevel::Medium => "MEDIUM",
            crate::domain::RiskLevel::High => "HIGH",
        };
        match risk_str {
            "LOW" | "MEDIUM" | "HIGH" => {} // Valid risk level
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

    // Validate diff_refs point to valid hunks in the canonical diff
    let diff_index = DiffIndex::new(diff_text)?;
    let warnings = Vec::new();

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
                        anyhow::bail!(
                            "Task {} references invalid hunk in {} at old_start={}, new_start={}. Nearest hunk: {:?}. Copy coordinates from the hunk manifest.",
                            task.id,
                            diff_ref.file,
                            hunk_ref.old_start,
                            hunk_ref.new_start,
                            diff_index_err.nearest()
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
