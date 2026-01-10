use super::config::ServerConfig;
use super::logging::log_to_file;
use super::parsing::parse_task;
use super::run_context::RunContext;
use crate::domain::{DiffRef, HunkRef, ReviewTask};
use crate::infra::db::{Database, ReviewRepository, TaskRepository};
use crate::infra::diff::index::DiffIndex;
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;

/// Validate raw hunk objects before parsing so we fail fast on malformed payloads.
fn validate_raw_task_hunks(raw_task: &Value, diff_index: &DiffIndex) -> Result<()> {
    // Validate hunk_ids format if present and validate they exist
    if let Some(hunk_ids) = raw_task.get("hunk_ids").and_then(|v| v.as_array()) {
        for hunk_id in hunk_ids {
            let hunk_id_str = hunk_id.as_str().ok_or_else(|| {
                anyhow::anyhow!("hunk_ids must contain strings, got: {}", hunk_id)
            })?;
            if !hunk_id_str.contains('#') {
                anyhow::bail!(
                    "hunk_id '{}' must contain '#' separator. Format: 'path/to/file#H1'",
                    hunk_id_str
                );
            }
            // Validate hunk exists in diff index
            if diff_index.get_hunk_coords(hunk_id_str).is_none() {
                anyhow::bail!(
                    "hunk_id '{}' does not exist in the diff manifest. Check the hunk manifest above for valid hunk IDs (format: 'path/to/file#H1').",
                    hunk_id_str
                );
            }
        }
    }

    // Validate diff_refs format if present (legacy support)
    let diff_refs = match raw_task.get("diff_refs").and_then(|v| v.as_array()) {
        Some(diff_refs) => diff_refs,
        None => return Ok(()),
    };

    for diff_ref in diff_refs {
        let hunks = match diff_ref.get("hunks") {
            Some(hunks) => hunks
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("diff_refs.hunks must be an array of objects"))?,
            None => continue,
        };

        for hunk in hunks {
            let obj = hunk
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("diff_refs.hunks entries must be objects"))?;
            let has_fields = obj.contains_key("old_start")
                && obj.contains_key("old_lines")
                && obj.contains_key("new_start")
                && obj.contains_key("new_lines");
            if !has_fields {
                anyhow::bail!(
                    "diff_refs.hunks entries must include old_start, old_lines, new_start, new_lines"
                );
            }
        }
    }

    Ok(())
}

fn validate_task_references(task: &ReviewTask, diff_index: &DiffIndex) -> Result<()> {
    if task.diff_refs.is_empty() {
        anyhow::bail!(
            "Task {} is missing diff references. The task must have either:\n\
             - diff_refs: [file path and hunk coordinates], OR\n\
             - hunk_ids: ['path/to/file#H1'] referencing hunks from the manifest.\n\
             If sending hunk_ids, they will be automatically converted to diff_refs.",
            task.id
        );
    }

    for diff_ref in &task.diff_refs {
        let file = diff_ref.file.as_str();
        if file.trim().is_empty() {
            anyhow::bail!(
                "Task {} has an empty diff_ref file. Use file paths from hunk_manifest_json.",
                task.id
            );
        }
        if file != file.trim() {
            anyhow::bail!(
                "Task {} has whitespace in diff_ref file '{}'. Copy file paths exactly from hunk_manifest_json.",
                task.id,
                file
            );
        }
        if file.starts_with("a/") || file.starts_with("b/") {
            anyhow::bail!(
                "Task {} diff_ref file '{}' must not include a/ or b/ prefixes. Use file paths from hunk_manifest_json.",
                task.id,
                file
            );
        }

        if diff_ref.hunks.is_empty() {
            diff_index.validate_file_exists(file)?;
            continue;
        }

        for hunk_ref in &diff_ref.hunks {
            diff_index.validate_hunk_exists(file, hunk_ref)?;
        }
    }

    Ok(())
}

fn convert_hunk_ids_to_diff_refs(
    diff_index: &DiffIndex,
    hunk_ids: &[String],
    task_id: &str,
) -> Result<Vec<DiffRef>> {
    let mut hunks_by_file: HashMap<String, Vec<HunkRef>> = HashMap::new();

    for hunk_id in hunk_ids {
        let coords = diff_index
            .get_hunk_coords(hunk_id)
            .ok_or_else(|| anyhow!("Task {} references invalid hunk_id '{}'.", task_id, hunk_id))?;

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

fn validate_task_diagram(task: &ReviewTask) -> Result<()> {
    let has_diagram = task
        .diagram
        .as_ref()
        .is_some_and(|diagram| !diagram.trim().is_empty());
    if !has_diagram {
        anyhow::bail!(
            "Task {} missing diagram. Every task must include a diagram.",
            task.id
        );
    }

    Ok(())
}

pub(super) fn open_database(config: &ServerConfig) -> Result<Database> {
    if let Some(path) = &config.db_path {
        Database::open_at(path.clone()).context("open database")
    } else {
        Database::open().context("open database")
    }
}

pub(super) fn save_task(config: &ServerConfig, raw_task: Value) -> Result<ReviewTask> {
    let ctx = load_run_context(config);
    let diff_index = DiffIndex::new(&ctx.diff_text)?;

    // Log raw task for debugging
    let raw_task_str = raw_task.to_string();
    let raw_task_preview = if raw_task_str.len() > 200 {
        format!("{}... (truncated)", &raw_task_str[..200])
    } else {
        raw_task_str
    };
    log_to_file(
        config,
        &format!("save_task received raw_task: {}", raw_task_preview),
    );

    // Verify the structural integrity of hunk data before proceeding with
    // database operations. This prevents storing malformed or incomplete tasks.
    validate_raw_task_hunks(&raw_task, &diff_index)?;

    let db = open_database(config)?;
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
        status: crate::domain::ReviewRunStatus::Running,
        created_at: ctx
            .created_at
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
    };

    // Upsert the parent review to ensure data consistency. Repository `save`
    // operations are idempotent and non-destructive.
    let review_repo = crate::infra::db::ReviewRepository::new(conn.clone());
    let review = crate::domain::Review {
        id: ctx.review_id.clone(),
        title: ctx
            .initial_title
            .clone()
            .unwrap_or_else(|| "Untitled Review".to_string()),
        summary: None, // Always start with no summary; it's added by finalize_review
        source: ctx.source.clone(),
        active_run_id: Some(ctx.run_id.clone()),
        status: crate::domain::ReviewStatus::Todo,
        created_at: ctx
            .created_at
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    review_repo
        .save(&review)
        .with_context(|| format!("save review {}", ctx.review_id))?;

    // Always update the active run and updated_at timestamp
    review_repo
        .set_active_run(&ctx.review_id, &ctx.run_id)
        .with_context(|| format!("set active run for review {}", ctx.review_id))?;

    review_run_repo
        .save(&review_run)
        .with_context(|| format!("save review run {}", ctx.run_id))?;

    // Convert hunk_ids to diff_refs if present and diff_refs is empty
    let raw_task = if raw_task.get("hunk_ids").is_some() && raw_task.get("diff_refs").is_none() {
        let diff_index = DiffIndex::new(&ctx.diff_text)?;
        let hunk_ids: Option<Vec<String>> = raw_task
            .get("hunk_ids")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            });

        if let Some(hunk_ids) = hunk_ids {
            if !hunk_ids.is_empty() {
                let diff_refs = convert_hunk_ids_to_diff_refs(&diff_index, &hunk_ids, "task")?;
                let mut updated = raw_task;
                updated["diff_refs"] = serde_json::to_value(&diff_refs)?;
                updated
            } else {
                raw_task
            }
        } else {
            raw_task
        }
    } else {
        raw_task
    };

    let mut task = parse_task(raw_task.clone())?;
    task.run_id = ctx.run_id.clone();

    let diff_index = DiffIndex::new(&ctx.diff_text)?;
    validate_task_references(&task, &diff_index)?;
    validate_task_diagram(&task)?;

    // Always set files from the provided diff_refs
    let mut files = Vec::new();
    for diff_ref in &task.diff_refs {
        if !files.contains(&diff_ref.file) {
            files.push(diff_ref.file.clone());
        }
    }
    task.files = files;

    let (additions, deletions) = diff_index.task_stats(&task.diff_refs)?;
    task.stats.additions = additions;
    task.stats.deletions = deletions;

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

    if title.is_none() && summary.is_none() {
        return Ok(());
    }

    let ctx = load_run_context(config);
    let db = open_database(config)?;
    let conn = db.connection();
    let review_repo = ReviewRepository::new(conn.clone());
    let review_run_repo = crate::infra::db::ReviewRunRepository::new(conn.clone());

    // Ensure the review record exists before updating its metadata.
    let review_placeholder = crate::domain::Review {
        id: ctx.review_id.clone(),
        title: ctx
            .initial_title
            .clone()
            .unwrap_or_else(|| "Untitled Review".to_string()),
        summary: None,
        source: ctx.source.clone(),
        active_run_id: Some(ctx.run_id.clone()),
        status: crate::domain::ReviewStatus::Todo,
        created_at: ctx
            .created_at
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    review_repo.save(&review_placeholder)?;

    // Fetch the review's current title if a new one isn't provided
    let review_title = if let Some(t) = title {
        t
    } else {
        review_repo
            .find_by_id(&ctx.review_id)?
            .map(|r| r.title)
            .unwrap_or(review_placeholder.title)
    };

    // Update the review with the new metadata
    review_repo
        .set_active_run(&ctx.review_id, &ctx.run_id)
        .with_context(|| format!("set active run for review {}", ctx.review_id))?;
    review_repo
        .update_title_and_summary(&ctx.review_id, &review_title, summary.as_deref())
        .context("update review title and summary")?;

    let review_run = crate::domain::ReviewRun {
        id: ctx.run_id.clone(),
        review_id: ctx.review_id.clone(),
        agent_id: ctx.agent_id.clone(),
        input_ref: ctx.input_ref.clone(),
        diff_text: ctx.diff_text.clone(),
        diff_hash: ctx.diff_hash.clone(),
        status: crate::domain::ReviewRunStatus::Completed,
        created_at: ctx
            .created_at
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
    };
    review_run_repo
        .save(&review_run)
        .with_context(|| format!("save review run {}", ctx.run_id))?;

    if let Err(err) =
        review_run_repo.update_status(&ctx.run_id, crate::domain::ReviewRunStatus::Completed)
    {
        log_to_file(
            config,
            &format!(
                "Failed to update run status to completed for {}: {}",
                ctx.run_id, err
            ),
        );
    }

    Ok(())
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
        diff_text: String::new().into(),
        diff_hash: String::new(),
        source: crate::domain::ReviewSource::DiffPaste {
            diff_hash: String::new(),
        },
        initial_title: Some("Review".to_string()),
        created_at: Some(Utc::now().to_rfc3339()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_raw_task_hunks_invalid() {
        let diff = r#"diff --git a/src/main.rs b/src/main.rs
index 1234567..89abcdef 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
+    println!("hello");
     println!("world");
 }
"#;
        let diff_index = DiffIndex::new(diff).unwrap();

        // Valid hunk_id that exists in the diff (uses original path with /)
        let raw = json!({
            "hunk_ids": ["src/main.rs#H1"]
        });
        assert!(validate_raw_task_hunks(&raw, &diff_index).is_ok());

        // Invalid format (no #)
        let raw = json!({
            "hunk_ids": ["invalid"]
        });
        assert!(validate_raw_task_hunks(&raw, &diff_index).is_err());

        // Non-existent hunk
        let raw = json!({
            "hunk_ids": ["src/main.rs#H99"]
        });
        assert!(validate_raw_task_hunks(&raw, &diff_index).is_err());

        // Invalid diff_refs structure
        let raw = json!({
            "diff_refs": [
                { "file": "a.rs", "hunks": [ { "old_start": 1 } ] }
            ]
        });
        assert!(validate_raw_task_hunks(&raw, &diff_index).is_err());
    }

    #[test]
    fn test_validate_task_references_prefixes() {
        let diff_index = DiffIndex::new("").unwrap();
        let mut task = ReviewTask {
            id: "t1".into(),
            run_id: "r1".into(),
            title: "T".into(),
            description: "D".into(),
            files: vec![],
            stats: Default::default(),
            diff_refs: vec![crate::domain::DiffRef {
                file: "a/b.rs".into(),
                hunks: vec![],
            }],
            insight: None,
            diagram: None,
            ai_generated: true,
            status: crate::domain::ReviewStatus::Todo,
            sub_flow: None,
        };
        // Should bail because of a/ prefix
        assert!(validate_task_references(&task, &diff_index).is_err());

        task.diff_refs[0].file = "b/b.rs".into();
        assert!(validate_task_references(&task, &diff_index).is_err());
    }
}
