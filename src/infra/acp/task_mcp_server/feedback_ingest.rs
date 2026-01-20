use super::config::ServerConfig;
use super::logging::log_to_file;
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

const DEFAULT_TITLE_TRUNCATION_LENGTH: usize = 50;

pub(super) fn save_agent_comment(config: &ServerConfig, args: Value) -> Result<String> {
    let hunk_id = args.get("hunk_id").and_then(|v| v.as_str());
    let line_id = args.get("line_id").and_then(|v| v.as_str());
    let line_content = args
        .get("line_content")
        .or(args.get("line"))
        .and_then(|v| v.as_str());

    // Priority: line_id > line_content > file+line (legacy)
    if let Some(hunk_id) = hunk_id {
        if let Some(line_id) = line_id {
            // Preferred: use hunk_id + line_id (e.g., "L3")
            return save_by_line_id(config, hunk_id, line_id, &args);
        } else if let Some(content) = line_content {
            // Fallback: use hunk_id + line_content
            return save_by_content(config, hunk_id, content, &args);
        }
    }

    // Legacy: use file + line number
    save_by_file_and_line(config, &args)
}

fn extract_rule_id(args: &Value) -> Option<String> {
    let raw = args
        .get("rule_id")
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())?;

    let normalized = raw.rsplit_once('|').map(|(_, id)| id.trim()).unwrap_or(raw);

    if normalized.is_empty() {
        None
    } else {
        Some(normalized.to_string())
    }
}

/// Save feedback using a simple line ID (e.g., "L3").
/// This is the preferred method as it requires no string matching.
fn save_by_line_id(
    config: &ServerConfig,
    hunk_id: &str,
    line_id: &str,
    args: &Value,
) -> Result<String> {
    let file = args
        .get("file")
        .and_then(|v| v.as_str())
        .or_else(|| hunk_id.rsplit_once('#').map(|(path, _)| path))
        .ok_or_else(|| anyhow!("Could not determine file path from hunk_id"))?;

    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing body"))?;

    if body.trim().is_empty() {
        return Err(anyhow!("feedback body cannot be empty"));
    }

    let side_str = args.get("side").and_then(|v| v.as_str()).unwrap_or("new");
    let side = match side_str.to_lowercase().as_str() {
        "old" => FeedbackSide::Old,
        _ => FeedbackSide::New,
    };

    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let end = body
                .char_indices()
                .map(|(i, _)| i)
                .nth(DEFAULT_TITLE_TRUNCATION_LENGTH)
                .unwrap_or(body.len());
            body[..end].to_string()
        });

    let impact_str = args
        .get("impact")
        .and_then(|v| v.as_str())
        .unwrap_or("nitpick");

    let impact = match impact_str.to_lowercase().as_str() {
        "blocking" => FeedbackImpact::Blocking,
        "nice_to_have" | "nice-to-have" => FeedbackImpact::NiceToHave,
        _ => FeedbackImpact::Nitpick,
    };

    let input_task_id = args
        .get("task_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let rule_id = extract_rule_id(args);

    let ctx = load_run_context(config);
    let db = open_database(config)?;
    let conn = db.connection();
    let feedback_repo = FeedbackRepository::new(conn.clone());
    let comment_repo = CommentRepository::new(conn.clone());
    let task_repo = TaskRepository::new(conn.clone());

    let diff_index = DiffIndex::new(&ctx.diff_text)?;

    // Use simple line ID lookup - no string matching needed!
    let line_location = diff_index
        .find_line_by_id(hunk_id, line_id)
        .ok_or_else(|| {
            let hunk_line_count = diff_index
                .get_hunk_lines(hunk_id)
                .map(|lines| lines.len())
                .unwrap_or(0);
            anyhow!(
                "Invalid line_id '{}' for hunk {}. Valid line IDs are L1 to L{}.",
                line_id,
                hunk_id,
                hunk_line_count
            )
        })?;

    let line_number = match side {
        FeedbackSide::Old => line_location.old_line_number,
        FeedbackSide::New => line_location.new_line_number,
    }
    .ok_or_else(|| {
        let other_side = match side {
            FeedbackSide::Old => "new",
            FeedbackSide::New => "old",
        };
        anyhow!(
            "Line {} exists only on the {} side. Use side: \"{}\".",
            line_id,
            other_side,
            other_side
        )
    })?;

    let hunk_ref = diff_index
        .get_hunk_coords(hunk_id)
        .ok_or_else(|| anyhow!("Hunk {} not found in diff", hunk_id))?;

    let final_task_id = if let Some(id) = input_task_id {
        let tasks = task_repo.find_by_run(&ctx.run_id)?;
        if !tasks.iter().any(|t| t.id == id) {
            return Err(anyhow!("Task ID '{}' not found in current review run.", id));
        }
        Some(id)
    } else {
        let tasks = task_repo.find_by_run(&ctx.run_id)?;
        let matching_tasks: Vec<_> = tasks
            .iter()
            .filter(|t| is_line_covered_by_task(t, file, line_number, side))
            .collect();

        if !matching_tasks.is_empty() {
            Some(matching_tasks[0].id.clone())
        } else {
            log_to_file(
                config,
                &format!(
                    "Feedback on {}:{} (hunk {}, line {}) is outside all tasks - saving as unassigned",
                    file, line_number, hunk_id, line_id
                ),
            );
            None
        }
    };

    let feedback_id = Uuid::new_v4().to_string();
    let comment_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let feedback = Feedback {
        id: feedback_id.clone(),
        review_id: ctx.review_id.clone(),
        task_id: final_task_id,
        rule_id,
        finding_id: None,
        title,
        status: ReviewStatus::Todo,
        impact,
        anchor: Some(FeedbackAnchor {
            file_path: Some(file.to_string()),
            line_number: Some(line_number),
            side: Some(side),
            hunk_ref: Some(hunk_ref),
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

fn save_by_content(
    config: &ServerConfig,
    hunk_id: &str,
    line_content: &str,
    args: &Value,
) -> Result<String> {
    let file = args
        .get("file")
        .and_then(|v| v.as_str())
        .or_else(|| hunk_id.rsplit_once('#').map(|(path, _)| path))
        .ok_or_else(|| anyhow!("Could not determine file path from hunk_id"))?;

    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing body"))?;

    if body.trim().is_empty() {
        return Err(anyhow!("feedback body cannot be empty"));
    }

    let side_str = args.get("side").and_then(|v| v.as_str()).unwrap_or("new");
    let side = match side_str.to_lowercase().as_str() {
        "old" => FeedbackSide::Old,
        _ => FeedbackSide::New,
    };

    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let end = body
                .char_indices()
                .map(|(i, _)| i)
                .nth(DEFAULT_TITLE_TRUNCATION_LENGTH)
                .unwrap_or(body.len());
            body[..end].to_string()
        });

    let impact_str = args
        .get("impact")
        .and_then(|v| v.as_str())
        .unwrap_or("nitpick");

    let impact = match impact_str.to_lowercase().as_str() {
        "blocking" => FeedbackImpact::Blocking,
        "nice_to_have" | "nice-to-have" => FeedbackImpact::NiceToHave,
        _ => FeedbackImpact::Nitpick,
    };

    let input_task_id = args
        .get("task_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let rule_id = extract_rule_id(args);

    let ctx = load_run_context(config);
    let db = open_database(config)?;
    let conn = db.connection();
    let feedback_repo = FeedbackRepository::new(conn.clone());
    let comment_repo = CommentRepository::new(conn.clone());
    let task_repo = TaskRepository::new(conn.clone());

    let diff_index = DiffIndex::new(&ctx.diff_text)?;

    let (line_number, hunk_ref) =
        find_line_by_content_with_validation(&diff_index, hunk_id, line_content, side, file)?;

    let final_task_id = if let Some(id) = input_task_id {
        let tasks = task_repo.find_by_run(&ctx.run_id)?;
        if !tasks.iter().any(|t| t.id == id) {
            return Err(anyhow!("Task ID '{}' not found in current review run.", id));
        }
        Some(id)
    } else {
        let tasks = task_repo.find_by_run(&ctx.run_id)?;
        let matching_tasks: Vec<_> = tasks
            .iter()
            .filter(|t| is_line_covered_by_task(t, file, line_number, side))
            .collect();

        if !matching_tasks.is_empty() {
            Some(matching_tasks[0].id.clone())
        } else {
            log_to_file(
                config,
                &format!(
                    "Feedback on {}:{} (hunk {}) is outside all tasks - saving as unassigned",
                    file, line_number, hunk_id
                ),
            );
            None
        }
    };

    let feedback_id = Uuid::new_v4().to_string();
    let comment_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let feedback = Feedback {
        id: feedback_id.clone(),
        review_id: ctx.review_id.clone(),
        task_id: final_task_id,
        rule_id,
        finding_id: None,
        title,
        status: ReviewStatus::Todo,
        impact,
        anchor: Some(FeedbackAnchor {
            file_path: Some(file.to_string()),
            line_number: Some(line_number),
            side: Some(side),
            hunk_ref: Some(hunk_ref),
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

fn find_line_by_content_with_validation(
    diff_index: &DiffIndex,
    hunk_id: &str,
    line_content: &str,
    side: FeedbackSide,
    expected_file: &str,
) -> Result<(u32, crate::domain::HunkRef)> {
    let (file_path, _hunk_idx) = diff_index.parse_hunk_id(hunk_id)
        .ok_or_else(|| anyhow!(
            "Invalid hunk_id format: '{}'. Expected format: 'path/to/file#H1' (e.g., 'src/auth.rs#H3')",
            hunk_id
        ))?;

    if file_path != expected_file {
        return Err(anyhow!(
            "File path in hunk_id ('{}') does not match specified file ('{}')",
            file_path,
            expected_file
        ));
    }

    let line_match = diff_index
        .find_line_by_content_with_numbers(hunk_id, line_content)
        .ok_or_else(|| {
            let line_count = line_content.lines().count();
            let multi_line_note = if line_count > 1 {
                format!(
                    "\n\n**Note:** Your line_content contains {} lines. Please provide only ONE line from the hunk manifest.",
                    line_count
                )
            } else {
                String::new()
            };
            let available_lines = list_lines_in_hunk(diff_index, hunk_id);
            anyhow!(
                "Could not find line content in hunk {}.{}\n\n\
                 **To fix this:**\n\
                 1. Copy the line EXACTLY from the hunk manifest (including whitespace)\n\
                 2. Use only ONE line - do NOT include code blocks or multiple lines\n\
                 3. Make sure you're using the correct hunk_id\n\n\
                 **Available lines in {}:**\n{}",
                hunk_id,
                multi_line_note,
                hunk_id,
                available_lines
            )
        })?;

    let line_number = match side {
        FeedbackSide::Old => line_match.old_line_number,
        FeedbackSide::New => line_match.new_line_number,
    }
    .ok_or_else(|| {
        let other_side = match side {
            FeedbackSide::Old => "new",
            FeedbackSide::New => "old",
        };
        anyhow!(
            "Line exists only on the {} side of hunk {}. Use side: \"{}\".",
            other_side,
            hunk_id,
            other_side
        )
    })?;
    let position = line_match.position_in_hunk;

    let hunk_ref = diff_index
        .get_hunk_coords(hunk_id)
        .ok_or_else(|| anyhow!("Hunk {} not found in diff", hunk_id))?;

    match side {
        FeedbackSide::Old => {
            if line_number < hunk_ref.old_start
                || line_number >= hunk_ref.old_start + hunk_ref.old_lines
            {
                return Err(anyhow!(
                    "Line {} is not in the old file range of hunk {} (old lines: {}-{}).\n\
                      Use side: \"new\" for lines added or changed in the new file.\n\
                      (Note: This line exists in the new file at position {} within the hunk)",
                    line_number,
                    hunk_id,
                    hunk_ref.old_start,
                    hunk_ref.old_start + hunk_ref.old_lines - 1,
                    position + 1
                ));
            }
        }
        FeedbackSide::New => {
            if line_number < hunk_ref.new_start
                || line_number >= hunk_ref.new_start + hunk_ref.new_lines
            {
                return Err(anyhow!(
                    "Line {} is not in the new file range of hunk {} (new lines: {}-{}).\n\
                      Use side: \"old\" for context lines from the old file.\n\
                      (Note: This line exists in the old file at position {} within the hunk)",
                    line_number,
                    hunk_id,
                    hunk_ref.new_start,
                    hunk_ref.new_start + hunk_ref.new_lines - 1,
                    position + 1
                ));
            }
        }
    }

    Ok((line_number, hunk_ref))
}

fn list_lines_in_hunk(diff_index: &DiffIndex, hunk_id: &str) -> String {
    let lines = match diff_index.get_hunk_lines_with_numbers(hunk_id) {
        Some(l) => l,
        None => return format!("Invalid hunk_id: {}", hunk_id),
    };

    if lines.is_empty() {
        return format!("Hunk {} has no lines", hunk_id);
    }

    let mut output = Vec::new();
    for line in lines {
        let is_add = line.is_addition;
        let is_del = line.is_deletion;
        let prefix = if is_add {
            "+"
        } else if is_del {
            "-"
        } else {
            " "
        };
        let line_num = if is_add {
            line.new_line_number
        } else if is_del {
            line.old_line_number
        } else {
            line.new_line_number.or(line.old_line_number)
        };
        let line_num_display = match line_num {
            Some(num) => format!("{:>3}", num),
            None => " ??".to_string(),
        };
        output.push(format!(
            "  {} {} | {}",
            prefix, line_num_display, line.content
        ));
    }

    output.join("\n")
}

fn save_by_file_and_line(config: &ServerConfig, args: &Value) -> Result<String> {
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

    if body.trim().is_empty() {
        return Err(anyhow!("feedback body cannot be empty"));
    }
    let side_str = args.get("side").and_then(|v| v.as_str()).unwrap_or("new");
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let end = body
                .char_indices()
                .map(|(i, _)| i)
                .nth(DEFAULT_TITLE_TRUNCATION_LENGTH)
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
    let rule_id = extract_rule_id(args);

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
    let review_run_repo = crate::infra::db::ReviewRunRepository::new(conn.clone());

    // Ensure the review run exists in the database before saving the feedback
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

    // Upsert the parent review to ensure data consistency
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

    review_repo
        .set_active_run(&ctx.review_id, &ctx.run_id)
        .with_context(|| format!("set active run for review {}", ctx.review_id))?;

    review_run_repo
        .save(&review_run)
        .with_context(|| format!("save review run {}", ctx.run_id))?;

    let diff_index = DiffIndex::new(&ctx.diff_text)?;
    validate_line_in_diff(&diff_index, file, line, side)?;

    let final_task_id = if let Some(id) = input_task_id {
        let tasks = task_repo.find_by_run(&ctx.run_id)?;
        if !tasks.iter().any(|t| t.id == id) {
            return Err(anyhow!("Task ID '{}' not found in current review run.", id));
        }
        Some(id)
    } else {
        let tasks = task_repo.find_by_run(&ctx.run_id)?;
        let matching_tasks: Vec<_> = tasks
            .iter()
            .filter(|t| is_line_covered_by_task(t, file, line, side))
            .collect();

        if !matching_tasks.is_empty() {
            Some(matching_tasks[0].id.clone())
        } else {
            // Feedback outside all tasks - save as unassigned
            log_to_file(
                config,
                &format!(
                    "Feedback on {}:{} is outside all tasks - saving as unassigned",
                    file, line
                ),
            );
            None
        }
    };

    let feedback_id = Uuid::new_v4().to_string();
    let comment_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let feedback = Feedback {
        id: feedback_id.clone(),
        review_id: ctx.review_id.clone(),
        task_id: final_task_id,
        rule_id,
        finding_id: None,
        title,
        status: ReviewStatus::Todo,
        impact,
        anchor: Some(FeedbackAnchor {
            file_path: Some(file.to_string()),
            line_number: Some(line),
            side: Some(side),
            hunk_ref: None,
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
    line: u32,
    side: FeedbackSide,
) -> Result<()> {
    diff_index
        .validate_file_exists(file)
        .map_err(|e| anyhow!(e.to_string()))?;

    let found = diff_index.line_exists_in_file(file, line, side);

    if !found {
        let (old_ranges, new_ranges) = diff_index.get_hunk_ranges(file);

        let available_new: Vec<String> = new_ranges
            .iter()
            .map(|(start, end)| format!("{}-{}", start, end))
            .collect();

        let available_old: Vec<String> = old_ranges
            .iter()
            .map(|(start, end)| format!("{}-{}", start, end))
            .collect();

        return Err(anyhow!(
            "Line {} (side: {:?}) not found in any hunk of {}.\n\
             Available new file lines: {}\n\
             Available old file lines: {}",
            file,
            line,
            side,
            available_new.join(", "),
            available_old.join(", ")
        ));
    }

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
