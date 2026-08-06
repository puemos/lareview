use crate::application::review::export::{ExportData, ExportOptions, ReviewExporter};
use crate::domain::{Feedback, FeedbackImpact, Review, ReviewRunEvent, ReviewStatus, ReviewTask};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tauri::State;

#[tauri::command]
pub fn load_tasks(
    state: State<'_, AppState>,
    run_id: Option<String>,
) -> Result<Vec<ReviewTask>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tasks = if let Some(run_id) = run_id {
        db.get_tasks_by_run(&run_id).map_err(|e| e.to_string())?
    } else {
        db.get_all_tasks().map_err(|e| e.to_string())?
    };
    Ok(tasks)
}

#[tauri::command]
pub fn get_all_reviews(state: State<'_, AppState>) -> Result<Vec<ReviewState>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let reviews = db.get_all_reviews().map_err(|e| e.to_string())?;
    Ok(reviews)
}

#[tauri::command]
pub fn get_review_runs(
    state: State<'_, AppState>,
    review_id: String,
) -> Result<Vec<ReviewRunState>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let runs = db.get_review_runs(&review_id).map_err(|e| e.to_string())?;
    Ok(runs)
}

#[tauri::command]
pub fn get_review_run_events(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<Vec<ReviewRunEvent>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_review_run_events(&run_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_task_status(
    state: State<'_, AppState>,
    task_id: String,
    status: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let review_status = ReviewStatus::from_str(&status).unwrap_or(ReviewStatus::Todo);
    db.update_task_status(&task_id, review_status)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_review(state: State<'_, AppState>, review_id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    if let Some(review) = db.get_review(&review_id).map_err(|e| e.to_string())?
        && let Some(run_id) = review.active_run_id
        && let Some(run) = db
            .get_review_run_by_id(&run_id)
            .map_err(|e| e.to_string())?
        && run.status.is_active()
    {
        return Err("Cancel the running review before deleting it.".to_string());
    }
    db.review_repo()
        .delete(&review_id)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn export_review(
    state: State<'_, AppState>,
    review_id: String,
    format: String,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let review = db
        .get_review(&review_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Review not found".to_string())?;

    let active_run_id = review.active_run_id.clone().unwrap_or_default();
    let tasks = db
        .get_tasks_by_run(&active_run_id)
        .map_err(|e| e.to_string())?;

    let feedbacks = db
        .get_feedback_by_review(&review_id)
        .map_err(|e| e.to_string())?;

    match format.as_str() {
        "markdown" => generate_markdown_export(&review, &tasks, &feedbacks),
        _ => Ok("exported_content".to_string()),
    }
}

fn generate_markdown_export(
    review: &Review,
    tasks: &[ReviewTask],
    feedbacks: &[Feedback],
) -> Result<String, String> {
    let mut md = format!("# {}\n\n", review.title);

    if let Some(summary) = &review.summary {
        md.push_str(summary);
        md.push_str("\n\n");
    }

    md.push_str("## Tasks\n\n");
    for task in tasks {
        let status_icon = match task.status {
            ReviewStatus::Todo => "[]",
            ReviewStatus::InProgress => "[ ]",
            ReviewStatus::Done => "[x]",
            ReviewStatus::Ignored => "[-]",
        };
        let risk = task.stats.risk.to_string();
        md.push_str(&format!("{} **{}** ({})\n", status_icon, task.title, risk));
        if !task.description.is_empty() {
            md.push_str(&format!(
                "> {}\n",
                task.description
                    .lines()
                    .take(3)
                    .collect::<Vec<_>>()
                    .join("\n> ")
            ));
        }
        md.push('\n');
    }

    if !feedbacks.is_empty() {
        md.push_str("## Feedback\n\n");
        for fb in feedbacks {
            let impact_icon = match fb.impact {
                FeedbackImpact::Blocking => "🔴",
                FeedbackImpact::NiceToHave => "🟡",
                FeedbackImpact::Nitpick => "🟢",
            };
            md.push_str(&format!("{} **{}**\n", impact_icon, fb.title));
            if let Some(anchor) = &fb.anchor
                && let Some(path) = &anchor.file_path
            {
                let line = anchor.line_number.unwrap_or(0);
                md.push_str(&format!("> At `{}`:{}\n", path, line));
            }
            md.push('\n');
        }
    }

    Ok(md)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewState {
    pub id: String,
    pub title: String,
    pub summary: Option<String>,
    pub agent_id: Option<String>,
    pub task_count: usize,
    pub created_at: String,
    pub source: crate::domain::ReviewSource,
    pub status: String,
    #[serde(default)]
    pub active_run_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRunState {
    pub id: String,
    pub review_id: String,
    pub agent_id: String,
    pub input_ref: String,
    pub diff_text: String,
    pub status: String,
    pub created_at: String,
    pub task_count: usize,
}

#[tauri::command]
pub async fn export_review_markdown(
    state: State<'_, AppState>,
    review_id: String,
    selected_tasks: Vec<String>,
    selected_feedbacks: Vec<String>,
) -> Result<String, String> {
    let data = {
        let db = state.db.lock().map_err(|e| e.to_string())?;

        let review = db
            .get_review(&review_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Review not found".to_string())?;

        let active_run_id = review
            .active_run_id
            .clone()
            .ok_or_else(|| "Review has no active run".to_string())?;

        let run = db
            .get_review_run_by_id(&active_run_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Review run not found".to_string())?;

        let tasks = db
            .get_tasks_by_run(&active_run_id)
            .map_err(|e| e.to_string())?;

        let feedbacks = db
            .get_feedback_by_review(&review_id)
            .map_err(|e| e.to_string())?
            // Filter out ignored feedbacks from export
            .into_iter()
            .filter(|f| f.status != ReviewStatus::Ignored)
            .collect::<Vec<_>>();

        let mut comments = Vec::new();
        for f in &feedbacks {
            let f_comments = db
                .get_comments_for_feedback(&f.id)
                .map_err(|e| e.to_string())?;
            comments.extend(f_comments);
        }

        // Fetch merge confidence
        let merge_confidence = db
            .merge_confidence_repo()
            .find_by_run_id(&active_run_id)
            .ok()
            .flatten();

        ExportData {
            review,
            run,
            tasks,
            feedbacks,
            comments,
            merge_confidence,
        }
    };

    let options = ExportOptions {
        include_summary: true,
        include_stats: true,
        include_metadata: true,
        include_tasks: true,
        include_feedbacks: true,
        include_context_diff: true,
        include_toc: true,
        selected_tasks: Some(selected_tasks.into_iter().collect()),
        selected_feedbacks: Some(selected_feedbacks.into_iter().collect()),
    };

    let result = ReviewExporter::export_to_markdown(&data, &options)
        .await
        .map_err(|e| e.to_string())?;

    Ok(result.markdown)
}

// ============================================================================
// Issue Check Commands
// ============================================================================

/// Get issue checks for a review run
#[tauri::command]
pub fn get_issue_checks_for_run(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<Vec<IssueCheckWithFindings>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let checks_with_findings = db
        .issue_check_repo()
        .find_checks_with_findings(&run_id)
        .map_err(|e| e.to_string())?;

    Ok(checks_with_findings
        .into_iter()
        .map(|(check, findings)| IssueCheckWithFindings { check, findings })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueCheckWithFindings {
    #[serde(flatten)]
    pub check: IssueCheck,
    pub findings: Vec<IssueFinding>,
}

use crate::domain::{IssueCheck, IssueFinding};

// ============================================================================
// Merge Confidence Commands
// ============================================================================

use crate::domain::MergeConfidence;

/// Serializable version of MergeConfidence for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeConfidenceState {
    /// The confidence score (1.0-5.0)
    pub score: f32,
    /// The score rounded to nearest integer (1-5)
    pub score_rounded: u8,
    /// Human-readable label (e.g., "Very Confident", "Moderate")
    pub label: String,
    /// Recommendation message
    pub recommendation: String,
    /// Bullet point reasons explaining the score
    pub reasons: Vec<String>,
    /// When this assessment was computed (RFC3339)
    pub computed_at: String,
}

impl From<MergeConfidence> for MergeConfidenceState {
    fn from(mc: MergeConfidence) -> Self {
        Self {
            score: mc.score,
            score_rounded: mc.score_rounded(),
            label: mc.label().to_string(),
            recommendation: mc.recommendation().to_string(),
            reasons: mc.reasons,
            computed_at: mc.computed_at,
        }
    }
}

/// Get merge confidence for a review run (submitted by agent during review)
#[tauri::command]
pub fn get_merge_confidence(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<Option<MergeConfidenceState>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let confidence = db
        .merge_confidence_repo()
        .find_by_run_id(&run_id)
        .map_err(|e| e.to_string())?;

    Ok(confidence.map(Into::into))
}
