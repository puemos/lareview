use crate::domain::ReviewTask;
use crate::infra::diff_index::DiffIndex;
use anyhow::Result;
use std::collections::HashSet;

pub(super) fn validate_tasks_payload(
    tasks: &[ReviewTask],
    _raw_payload: Option<&serde_json::Value>, // Keep for interface compatibility but don't use
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
    }

    // Validate diff_refs point to valid hunks in the canonical diff
    let diff_index = DiffIndex::new(diff_text)?;
    let mut warnings = Vec::new();

    for task in tasks {
        // Validate each diff_ref points to a real hunk
        for diff_ref in &task.diff_refs {
            for hunk_ref in &diff_ref.hunks {
                match diff_index.validate_hunk_exists(diff_ref.file.as_str(), hunk_ref) {
                    Ok(_) => {} // Valid hunk reference
                    Err(err) => {
                        // If this is a DiffIndexError, we can get more details
                        if let Some(diff_index_err) =
                            err.downcast_ref::<crate::infra::diff_index::DiffIndexError>()
                        {
                            warnings.push(format!(
                                "Task {} references hunk in file {} that does not exist in diff. Nearest hunks: {:?}",
                                task.id, diff_ref.file, diff_index_err.nearest()
                            ));
                        } else {
                            warnings.push(format!(
                                "Task {} references hunk in file {} that does not exist in diff",
                                task.id, diff_ref.file
                            ));
                        }
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
