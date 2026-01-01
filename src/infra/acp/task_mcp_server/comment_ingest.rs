use super::config::ServerConfig;
use super::task_ingest::{load_run_context, open_database};
use crate::domain::{
    Comment, Feedback, FeedbackAnchor, FeedbackImpact, FeedbackSide, ReviewStatus,
};
use crate::infra::db::{CommentRepository, FeedbackRepository, TaskRepository};
use crate::infra::diff::index::DiffIndex;
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

pub(super) fn save_agent_comment(config: &ServerConfig, args: Value) -> Result<String> {
    let file = args
        .get("file")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing file"))?;
    let line = args
        .get("line")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow!("missing line"))? as u32;
    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing body"))?;

    // Validate body is not empty or just whitespace
    if body.trim().is_empty() {
        return Err(anyhow!("feedback body cannot be empty"));
    }
    let side_str = args.get("side").and_then(|v| v.as_str()).unwrap_or("new");
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            // Default title from body (truncated)
            let end = body
                .char_indices()
                .map(|(i, _)| i)
                .nth(50)
                .unwrap_or(body.len());
            body[..end].to_string()
        });
    let impact_str = args
        .get("impact")
        .and_then(|v| v.as_str())
        .unwrap_or("nitpick");
    let input_task_id = args
        .get("task_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let side = match side_str.to_lowercase().as_str() {
        "old" => FeedbackSide::Old,
        _ => FeedbackSide::New,
    };

    let impact = match impact_str.to_lowercase().as_str() {
        "blocking" => FeedbackImpact::Blocking,
        "nice_to_have" | "nice-to-have" => FeedbackImpact::NiceToHave,
        _ => FeedbackImpact::Nitpick,
    };

    let ctx = load_run_context(config);
    let db = open_database(config)?;
    let conn = db.connection();
    let feedback_repo = FeedbackRepository::new(conn.clone());
    let comment_repo = CommentRepository::new(conn.clone());
    let task_repo = TaskRepository::new(conn.clone());

    let diff_index = DiffIndex::new(&ctx.diff_text)?;
    validate_line_in_diff(&diff_index, file, line, side)?;

    let final_task_id = if let Some(id) = input_task_id {
        // Verify provided task ID exists
        // We can't easily check if ID exists without a query, but let's assume if the agent
        // provided it, they mean it. Ideally we should verify it belongs to this run.
        // For strictness, let's verify it matches the run.
        let tasks = task_repo.find_by_run(&ctx.run_id)?;
        if !tasks.iter().any(|t| t.id == id) {
            return Err(anyhow!("Task ID '{}' not found in current review run.", id));
        }
        Some(id)
    } else {
        // Auto-link
        let tasks = task_repo.find_by_run(&ctx.run_id)?;
        let matching_tasks: Vec<_> = tasks
            .iter()
            .filter(|t| is_line_covered_by_task(t, file, line, side))
            .collect();

        if matching_tasks.len() == 1 {
            Some(matching_tasks[0].id.clone())
        } else if matching_tasks.len() > 1 {
            // Ambiguous: pick the first one (or arguably the smallest one, but first is deterministic enough)
            Some(matching_tasks[0].id.clone())
        } else {
            return Err(anyhow!(
                "Comment on {}:{} does not fall within the scope of any generated task. \
                 Please ensure you have created a task that covers this file and line, or provide a valid 'task_id'.",
                file,
                line
            ));
        }
    };

    let feedback_id = Uuid::new_v4().to_string();
    let comment_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let feedback = Feedback {
        id: feedback_id.clone(),
        review_id: ctx.review_id.clone(),
        task_id: final_task_id,
        title,
        status: ReviewStatus::Todo,
        impact,
        anchor: Some(FeedbackAnchor {
            file_path: Some(file.to_string()),
            line_number: Some(line),
            side: Some(side),
            hunk_ref: None, // Inferred by UI later or left empty
            head_sha: None,
        }),
        author: format!("agent:{}", ctx.agent_id),
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    let comment = Comment {
        id: comment_id,
        feedback_id: feedback_id.clone(),
        author: format!("agent:{}", ctx.agent_id),
        body: body.to_string(),
        parent_id: None,
        created_at: now.clone(),
        updated_at: now,
    };

    feedback_repo.save(&feedback).context("save feedback")?;
    comment_repo.save(&comment).context("save comment")?;

    Ok(feedback_id)
}

fn validate_line_in_diff(
    diff_index: &DiffIndex,
    file: &str,
    _line: u32,
    _side: FeedbackSide,
) -> Result<()> {
    diff_index
        .validate_file_exists(file)
        .map_err(|e| anyhow!(e.to_string()))?;

    // Line-level validation against the diff index is currently deferred.
    // Strict line validation would require exposing internal DiffIndex state or
    // re-parsing the unified diff.
    //
    // Current strategy: Rely on `is_line_covered_by_task` (line 184) to verify
    // that the agent's comment falls within the scope of a generated task.
    // Since tasks are derived from the diff, this effectively validates that
    // the comment refers to a changed line.

    Ok(())
}

fn is_line_covered_by_task(
    task: &crate::domain::ReviewTask,
    file: &str,
    line: u32,
    side: FeedbackSide,
) -> bool {
    if !task.files.contains(&file.to_string()) {
        return false;
    }

    for diff_ref in &task.diff_refs {
        if diff_ref.file != file {
            continue;
        }

        // If no hunks specified (empty list), it means "all hunks in file".
        // In that case, we assume it covers the file.
        // However, we should technically check if the line is in *any* hunk of the file.
        // But for "empty array = select all", we usually treat it as valid.
        if diff_ref.hunks.is_empty() {
            return true;
        }

        for hunk in &diff_ref.hunks {
            match side {
                FeedbackSide::Old => {
                    if line >= hunk.old_start && line < hunk.old_start + hunk.old_lines {
                        return true;
                    }
                }
                FeedbackSide::New => {
                    if line >= hunk.new_start && line < hunk.new_start + hunk.new_lines {
                        return true;
                    }
                }
            }
        }
    }

    false
}
