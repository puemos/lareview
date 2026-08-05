use crate::domain::{
    Comment, Feedback, FeedbackAnchor, FeedbackImpact, FeedbackSide, ReviewStatus,
};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn save_feedback(
    state: State<'_, AppState>,
    feedback: FeedbackInput,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();

    let anchor = if let (Some(file_path), Some(line_number), Some(side)) =
        (feedback.file_path, feedback.line_number, feedback.side)
    {
        Some(FeedbackAnchor {
            file_path: Some(file_path),
            line_number: Some(line_number),
            side: if side == "old" {
                Some(FeedbackSide::Old)
            } else {
                Some(FeedbackSide::New)
            },
            hunk_ref: None,
            head_sha: None,
        })
    } else {
        None
    };

    let impact = FeedbackImpact::from_str(&feedback.impact).unwrap_or(FeedbackImpact::Nitpick);

    let feedback_domain = Feedback {
        id: id.clone(),
        review_id: feedback.review_id,
        task_id: feedback.task_id,
        rule_id: feedback.rule_id,
        finding_id: None,
        category: None,
        title: feedback.title,
        status: ReviewStatus::Todo,
        impact,
        confidence: 1.0, // User-created feedback is high confidence
        anchor,
        author: "user".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    db.save_feedback(&feedback_domain, &id)
        .map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn get_feedback_comments(
    state: State<'_, AppState>,
    feedback_id: String,
) -> Result<Vec<Comment>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let comments = db
        .get_comments_for_feedback(&feedback_id)
        .map_err(|e| e.to_string())?;
    Ok(comments)
}

#[tauri::command]
pub fn add_comment(
    state: State<'_, AppState>,
    feedback_id: String,
    body: String,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    let comment = Comment {
        id: id.clone(),
        feedback_id,
        author: "user".to_string(),
        body,
        parent_id: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    db.save_comment(&comment).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn update_feedback_status(
    state: State<'_, AppState>,
    feedback_id: String,
    status: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let review_status = ReviewStatus::from_str(&status).unwrap_or(ReviewStatus::Todo);

    // If status is being set to "ignored", record the rejection
    if review_status == ReviewStatus::Ignored {
        // Fetch feedback details for rejection tracking
        if let Ok(Some(feedback)) = db.feedback_repo().find_by_id(&feedback_id) {
            let rejection_repo = db.rejection_repo();

            // Only record if not already recorded
            if !rejection_repo
                .rejection_exists(&feedback_id)
                .unwrap_or(false)
            {
                // Extract agent_id from author (format: "agent:agent_id")
                let agent_id = feedback
                    .author
                    .strip_prefix("agent:")
                    .unwrap_or(&feedback.author)
                    .to_string();

                // Extract file extension from anchor if available
                let file_extension = feedback
                    .anchor
                    .as_ref()
                    .and_then(|a| a.file_path.as_ref())
                    .and_then(|p| {
                        std::path::Path::new(p)
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(|s| s.to_string())
                    });

                let _ = rejection_repo.record_rejection(
                    &feedback_id,
                    &feedback.review_id,
                    feedback.rule_id.as_deref(),
                    &agent_id,
                    &feedback.impact.to_string(),
                    feedback.confidence,
                    file_extension.as_deref(),
                    &feedback.title,
                );
            }
        }
    }

    db.update_feedback_status(&feedback_id, review_status)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn update_feedback_impact(
    state: State<'_, AppState>,
    feedback_id: String,
    impact: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let feedback_impact = FeedbackImpact::from_str(&impact).unwrap_or(FeedbackImpact::Nitpick);
    db.update_feedback_impact(&feedback_id, feedback_impact)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_feedback(state: State<'_, AppState>, feedback_id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_feedback(&feedback_id)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_feedback_by_review(
    state: State<'_, AppState>,
    review_id: String,
) -> Result<Vec<Feedback>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let feedbacks = db
        .get_feedback_by_review(&review_id)
        .map_err(|e| e.to_string())?;
    Ok(feedbacks)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DiffSnippetLine {
    pub line_number: u32,
    pub content: String,
    pub prefix: String,
    pub is_addition: bool,
    pub is_deletion: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FeedbackDiffSnippet {
    pub file_path: String,
    pub hunk_header: String,
    pub lines: Vec<DiffSnippetLine>,
    pub highlighted_line: Option<u32>,
}

#[tauri::command]
pub fn get_feedback_diff_snippet(
    state: State<'_, AppState>,
    feedback_id: String,
    context_lines: u32,
) -> Result<Option<FeedbackDiffSnippet>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let feedback = db
        .feedback_repo()
        .find_by_id(&feedback_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Feedback not found".to_string())?;

    let anchor = match &feedback.anchor {
        Some(a) => a,
        None => return Ok(None),
    };

    let file_path = match &anchor.file_path {
        Some(f) => f.clone(),
        None => return Ok(None),
    };

    let line_number = match anchor.line_number {
        Some(l) => l,
        None => return Ok(None),
    };

    let side = match anchor.side {
        Some(crate::domain::FeedbackSide::Old) => crate::domain::FeedbackSide::Old,
        _ => crate::domain::FeedbackSide::New,
    };

    let review_id = &feedback.review_id;
    let review = db
        .get_review(review_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Review not found".to_string())?;

    let active_run_id = match &review.active_run_id {
        Some(id) => id.clone(),
        None => return Ok(None),
    };

    let review_run = db
        .get_review_run_by_id(&active_run_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Review run not found".to_string())?;

    let diff_index = crate::infra::diff::index::DiffIndex::new(&review_run.diff_text)
        .map_err(|e| e.to_string())?;

    let file_index = match diff_index.files.get(&file_path) {
        Some(f) => f,
        None => return Ok(None),
    };

    let mut target_hunk: Option<&crate::infra::diff::index::IndexedHunk> = None;
    for indexed_hunk in &file_index.all_hunks {
        let coords = indexed_hunk.coords;
        let hunk = &indexed_hunk.hunk;
        match side {
            crate::domain::FeedbackSide::Old => {
                let old_end = coords.0 + hunk.source_length.saturating_sub(1) as u32;
                if line_number >= coords.0 && line_number <= old_end {
                    target_hunk = Some(indexed_hunk);
                    break;
                }
            }
            crate::domain::FeedbackSide::New => {
                let new_end = coords.1 + hunk.target_length.saturating_sub(1) as u32;
                if line_number >= coords.1 && line_number <= new_end {
                    target_hunk = Some(indexed_hunk);
                    break;
                }
            }
        }
    }

    let indexed_hunk = match target_hunk {
        Some(h) => h,
        None => return Ok(None),
    };

    let coords = indexed_hunk.coords;
    let hunk = &indexed_hunk.hunk;
    let hunk_header = format!(
        "@@ -{},{} +{},{} @@",
        coords.0, hunk.source_length, coords.1, hunk.target_length
    );

    let mut snippet_lines: Vec<DiffSnippetLine> = Vec::new();

    let start_in_hunk = match side {
        crate::domain::FeedbackSide::Old => {
            if line_number >= coords.0 {
                Some((line_number - coords.0) as usize)
            } else {
                Some(0)
            }
        }
        crate::domain::FeedbackSide::New => {
            if line_number >= coords.1 {
                Some((line_number - coords.1) as usize)
            } else {
                Some(0)
            }
        }
    };

    let context_start = start_in_hunk
        .map(|s| s.saturating_sub(context_lines as usize))
        .unwrap_or(0);

    let context_end = start_in_hunk
        .map(|s| std::cmp::min(s + context_lines as usize + 1, hunk.target_length))
        .unwrap_or(hunk.target_length);

    let coords_clone = coords;
    let mut current_line_in_hunk = 0;
    crate::infra::diff::index::DiffIndex::walk_hunk_lines(
        hunk,
        coords_clone,
        |_pos, line, old_num, new_num| {
            if current_line_in_hunk >= context_start && current_line_in_hunk < context_end {
                let is_add = line.line_type.as_str() == unidiff::LINE_TYPE_ADDED;
                let is_del = line.line_type.as_str() == unidiff::LINE_TYPE_REMOVED;
                let prefix = if is_add {
                    "+"
                } else if is_del {
                    "-"
                } else {
                    " "
                };
                let display_line_number = match side {
                    crate::domain::FeedbackSide::Old => old_num,
                    crate::domain::FeedbackSide::New => new_num,
                };

                snippet_lines.push(DiffSnippetLine {
                    line_number: display_line_number.unwrap_or(0),
                    content: line.value.trim_end().to_string(),
                    prefix: prefix.to_string(),
                    is_addition: is_add,
                    is_deletion: is_del,
                });
            }
            current_line_in_hunk += 1;
        },
    );

    let highlighted_line = match side {
        crate::domain::FeedbackSide::Old => anchor.line_number,
        crate::domain::FeedbackSide::New => anchor.line_number,
    };

    Ok(Some(FeedbackDiffSnippet {
        file_path,
        hunk_header,
        lines: snippet_lines,
        highlighted_line,
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackInput {
    pub review_id: String,
    pub task_id: Option<String>,
    pub rule_id: Option<String>,
    pub title: String,
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
    pub side: Option<String>,
    pub content: String,
    pub impact: String,
}
