use super::config::ServerConfig;
use super::logging;
use super::parsing::parse_task;
use super::run_context::RunContext;
use crate::domain::ReviewTask;
use crate::infra::db::{Database, ReviewRepository, TaskRepository};
use crate::infra::diff_index::DiffIndex;
use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::Value;

pub(super) fn save_task(config: &ServerConfig, raw_task: Value) -> Result<ReviewTask> {
    let ctx = load_run_context(config);

    let db = Database::open().context("open database")?;
    let conn = db.connection();
    let task_repo = TaskRepository::new(conn.clone());
    let review_run_repo = crate::infra::db::ReviewRunRepository::new(conn.clone());

    // Ensure the review run exists in the database before saving the task
    let review_run = crate::domain::ReviewRun {
        id: ctx.run_id.clone(),
        review_id: ctx.review_id.clone(),
        agent_id: ctx.agent_id.clone(),
        input_ref: ctx.input_ref.clone(),
        diff_text: ctx.diff_text.clone(),
        diff_hash: ctx.diff_hash.clone(),
        created_at: ctx
            .created_at
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
    };

    // Also ensure the parent review exists
    let review_repo = crate::infra::db::ReviewRepository::new(conn.clone());
    let review = crate::domain::Review {
        id: ctx.review_id.clone(),
        title: ctx
            .initial_title
            .clone()
            .unwrap_or_else(|| "Untitled Review".to_string()),
        summary: None,
        source: ctx.source.clone(),
        active_run_id: Some(ctx.run_id.clone()),
        created_at: ctx
            .created_at
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
        updated_at: ctx
            .created_at
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
    };

    review_repo
        .save(&review)
        .with_context(|| format!("save review {}", ctx.review_id))?;

    review_run_repo
        .save(&review_run)
        .with_context(|| format!("save review run {}", ctx.run_id))?;

    let mut task = parse_task(raw_task.clone())?;
    task.run_id = ctx.run_id.clone();

    // Process diff_refs to handle both hunk IDs and numeric coordinates
    if !task.diff_refs.is_empty() {
        // Always set files from the provided diff_refs
        let mut files = Vec::new();
        for diff_ref in &task.diff_refs {
            if !files.contains(&diff_ref.file) {
                files.push(diff_ref.file.clone());
            }
        }
        task.files = files;

        // Check if any hunks in the raw JSON args are strings (hunk IDs) rather than objects (coordinates)
        // This needs to be done by parsing the raw JSON differently depending on the type
        let resolved_diff_refs =
            match resolve_hunk_ids_to_numeric(&raw_task.clone(), &ctx.diff_text) {
                Ok(resolved) => resolved,
                Err(err) => {
                    // Log the error but continue with original diff_refs
                    logging::log_to_file(
                        config,
                        &format!("Failed to resolve hunk IDs, using original: {err:?}"),
                    );
                    for cause in err.chain() {
                        logging::log_to_file(config, &format!("  Caused by: {cause}"));
                    }
                    task.diff_refs.clone()
                }
            };

        // Best-effort stats calculation. If the agent's hunk coordinates don't line up
        // with the canonical diff, log and fall back to the provided stats instead of
        // failing the whole tool call.
        match DiffIndex::new(&ctx.diff_text) {
            Ok(diff_index) => match diff_index.task_stats(&resolved_diff_refs) {
                Ok((additions, deletions)) => {
                    task.stats.additions = additions;
                    task.stats.deletions = deletions;
                    // Use the resolved diff_refs instead of the original
                    task.diff_refs = resolved_diff_refs;
                }
                Err(err) => {
                    // Log detailed error if it's a DiffIndexError
                    if let Some(diff_index_err) =
                        err.downcast_ref::<crate::infra::diff_index::DiffIndexError>()
                    {
                        logging::log_to_file(
                            config,
                            &format!(
                                "return_task: diff_refs mismatch; using agent-provided stats. Detailed error: {}",
                                diff_index_err.to_json()
                            ),
                        );
                    } else {
                        logging::log_to_file(
                            config,
                            &format!(
                                "return_task: diff_refs mismatch; using agent-provided stats. Error: {err:?}"
                            ),
                        );
                        for cause in err.chain() {
                            logging::log_to_file(config, &format!("  Caused by: {cause}"));
                        }
                    }
                    // Continue with the task save even if stats calculation failed
                    task.diff_refs = resolved_diff_refs; // Still use resolved diff_refs
                }
            },
            Err(err) => {
                logging::log_to_file(
                    config,
                    &format!(
                        "return_task: failed to build DiffIndex; using agent-provided stats. Error: {err:?}"
                    ),
                );
                for cause in err.chain() {
                    logging::log_to_file(config, &format!("  Caused by: {cause}"));
                }
                // Continue with the task save using the resolved diff_refs
                task.diff_refs = resolved_diff_refs;
            }
        }
    }

    task_repo
        .save(&task)
        .with_context(|| format!("save task {}", task.id))?;

    Ok(task)
}

pub(super) fn update_review_metadata(config: &ServerConfig, args: Value) -> Result<()> {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let summary = args
        .get("summary")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let db = Database::open().context("open database")?;
    let conn = db.connection();
    let review_repo = ReviewRepository::new(conn.clone());
    let review_run_repo = crate::infra::db::ReviewRunRepository::new(conn.clone());

    if let Some(title) = title {
        // Note: run_context_id is not available in ServerConfig, so we'll use ctx.review_id from load_run_context
        let ctx = load_run_context(config);

        // Also ensure the parent review exists
        let review = crate::domain::Review {
            id: ctx.review_id.clone(),
            title: ctx
                .initial_title
                .clone()
                .unwrap_or_else(|| "Untitled Review".to_string()),
            summary: None,
            source: ctx.source.clone(),
            active_run_id: Some(ctx.run_id.clone()),
            created_at: ctx
                .created_at
                .clone()
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
            updated_at: ctx
                .created_at
                .clone()
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
        };

        let review_repo_for_save = crate::infra::db::ReviewRepository::new(conn.clone());
        review_repo_for_save
            .save(&review)
            .with_context(|| format!("save review {}", ctx.review_id))?;

        // Ensure the review run exists in the database
        let review_run = crate::domain::ReviewRun {
            id: ctx.run_id.clone(),
            review_id: ctx.review_id.clone(),
            agent_id: ctx.agent_id.clone(),
            input_ref: ctx.input_ref.clone(),
            diff_text: ctx.diff_text.clone(),
            diff_hash: ctx.diff_hash.clone(),
            created_at: ctx
                .created_at
                .clone()
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
        };
        review_run_repo
            .save(&review_run)
            .with_context(|| format!("save review run {}", ctx.run_id))?;

        review_repo
            .update_title_and_summary(&ctx.review_id, &title, summary.as_deref())
            .context("update review title and summary")?;
    }

    Ok(())
}

/// Resolve hunk IDs to numeric coordinates in the raw JSON task payload
/// This function implements tolerant validation and repair of diff_refs
fn resolve_hunk_ids_to_numeric(
    raw_task: &Value,
    diff_text: &str,
) -> Result<Vec<crate::domain::DiffRef>> {
    // Get the diff_refs array from the raw task
    let diff_refs_array = raw_task
        .get("diff_refs")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("diff_refs not found or not an array"))?;

    let diff_index = crate::infra::diff_index::DiffIndex::new(diff_text)?;
    let mut resolved_diff_refs = Vec::new();

    for diff_ref_value in diff_refs_array {
        let raw_file = diff_ref_value
            .get("file")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("file not found in diff_ref"))?
            .to_string();

        // Normalize the file path consistently
        let normalized_file = crate::infra::diff::normalize_task_path(&raw_file);

        let hunks_array = diff_ref_value.get("hunks").and_then(|v| v.as_array());

        // If hunks is missing or not an array, treat as empty array
        let hunks_array = match hunks_array {
            Some(arr) => arr,
            None => &Vec::new(), // empty array
        };

        let mut resolved_hunks = Vec::new();
        let mut string_hunk_ids = Vec::new();
        let mut numeric_hunk_objects = Vec::new();
        let mut invalid_hunks = Vec::new(); // Track invalid hunks for warnings

        // Separate string hunk IDs from numeric hunk objects
        for hunk_value in hunks_array {
            if let Some(hunk_id) = hunk_value.as_str() {
                // This is a hunk ID (string)
                string_hunk_ids.push(hunk_id.to_string());
            } else if hunk_value.is_object() {
                // This is a numeric hunk object, try to parse it
                match serde_json::from_value::<crate::domain::HunkRef>(hunk_value.clone()) {
                    Ok(hunk_ref) => {
                        numeric_hunk_objects.push(hunk_ref);
                    }
                    Err(_) => {
                        // Invalid hunk object, track for warning
                        invalid_hunks.push(hunk_value.clone());
                    }
                }
            } else {
                // Invalid hunk type, track for warning
                invalid_hunks.push(hunk_value.clone());
            }
        }

        // Handle empty hunks as "all hunks for this file" (Option A implementation)
        if hunks_array.is_empty() {
            // Get all available hunks for this file
            let all_hunks = diff_index.available_hunks(&normalized_file);
            resolved_hunks.extend(all_hunks);
        } else {
            // Resolve string hunk IDs to numeric coordinates (tolerant: skip invalid IDs)
            if !string_hunk_ids.is_empty() {
                for hunk_id in &string_hunk_ids {
                    match diff_index.resolve_hunk_id(&normalized_file, hunk_id) {
                        Ok(hunk_ref) => resolved_hunks.push(hunk_ref),
                        Err(_) => {
                            // Skip invalid hunk ID and log warning
                            // We can't log here since we don't have config access, so we'll just continue
                            // In a real implementation, we might pass warnings upward
                        }
                    }
                }
            }

            // Add the numeric hunk objects as-is (some may have been skipped due to parsing errors)
            resolved_hunks.extend(numeric_hunk_objects);
        }

        // If we couldn't resolve any valid hunks for this file and it exists in the diff,
        // use all hunks as a fallback to ensure the task doesn't become completely invalid
        if resolved_hunks.is_empty() && !diff_index.available_hunks(&normalized_file).is_empty() {
            let all_hunks = diff_index.available_hunks(&normalized_file);
            resolved_hunks = all_hunks;
        }

        resolved_diff_refs.push(crate::domain::DiffRef {
            file: normalized_file,
            hunks: resolved_hunks,
        });
    }

    Ok(resolved_diff_refs)
}

pub(super) fn load_run_context(config: &ServerConfig) -> RunContext {
    if let Some(path) = &config.run_context
        && let Ok(content) = std::fs::read_to_string(path)
        && let Ok(ctx) = serde_json::from_str::<RunContext>(&content)
    {
        return ctx;
    }

    RunContext {
        review_id: "local-review".to_string(),
        run_id: "local-run".to_string(),
        agent_id: "unknown".to_string(),
        input_ref: "unknown".to_string(),
        diff_text: String::new(),
        diff_hash: String::new(),
        source: crate::domain::ReviewSource::DiffPaste {
            diff_hash: String::new(),
        },
        initial_title: Some("Review".to_string()),
        created_at: Some(Utc::now().to_rfc3339()),
    }
}
