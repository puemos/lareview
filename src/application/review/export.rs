use crate::domain::{
    Comment, Feedback, FeedbackImpact, Review, ReviewRun, ReviewStatus, ReviewTask, RiskLevel,
};
use crate::infra::d2::d2_to_ascii_async;
use anyhow::Result;
use futures::future::join_all;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ExportData {
    pub review: Review,
    pub run: ReviewRun,
    pub tasks: Vec<ReviewTask>,
    pub feedbacks: Vec<Feedback>,
    pub comments: Vec<Comment>,
}

#[derive(Debug, Clone)]
pub struct ExportResult {
    pub markdown: String,
    pub assets: std::collections::HashMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportOptions {
    pub include_summary: bool,
    pub include_stats: bool,
    pub include_metadata: bool,
    pub include_tasks: bool,
    pub include_feedbacks: bool,
    pub include_feedback_ids: Option<std::collections::HashSet<String>>,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            include_summary: true,
            include_stats: true,
            include_metadata: true,
            include_tasks: true,
            include_feedbacks: true,
            include_feedback_ids: None,
        }
    }
}

pub struct ReviewExporter;

impl ReviewExporter {
    pub async fn export_to_markdown(
        data: &ExportData,
        options: &ExportOptions,
    ) -> Result<ExportResult> {
        let mut md = String::new();
        let assets = std::collections::HashMap::new();

        // 1. Prepare diagrams to render (ASCII for everything)
        let mut diagrams_to_render: Vec<Arc<str>> = Vec::new();
        if options.include_tasks || options.include_feedbacks {
            for task in &data.tasks {
                if let Some(diagram_code) = &task.diagram
                    && !diagrams_to_render.contains(diagram_code)
                {
                    diagrams_to_render.push(diagram_code.clone());
                }
            }
        }

        // 2. Render all diagrams in parallel as ASCII
        let render_tasks = diagrams_to_render.iter().map(|code| {
            let code_clone = code.clone();
            async move { (code_clone.clone(), d2_to_ascii_async(&code_clone).await) }
        });

        let render_results: HashMap<Arc<str>, Result<String, String>> =
            join_all(render_tasks).await.into_iter().collect();

        // 3. Generate Markdown
        // Title
        md.push_str(&format!("# {}\n\n", data.review.title));

        if options.include_summary
            && let Some(summary) = &data.review.summary
        {
            md.push_str(&format!("{}\n\n", summary));
        }

        if options.include_stats {
            // Stats Overview
            md.push_str("## Overview\n\n");
            let total_tasks = data.tasks.len();
            let completed_tasks = data
                .tasks
                .iter()
                .filter(|t| t.status == ReviewStatus::Done)
                .count();
            let high_risk = data
                .tasks
                .iter()
                .filter(|t| t.stats.risk == RiskLevel::High)
                .count();
            let medium_risk = data
                .tasks
                .iter()
                .filter(|t| t.stats.risk == RiskLevel::Medium)
                .count();
            let low_risk = data
                .tasks
                .iter()
                .filter(|t| t.stats.risk == RiskLevel::Low)
                .count();

            md.push_str("| Metric | Value |\n| :--- | :--- |\n");
            md.push_str(&format!("| **Total Tasks** | {} |\n", total_tasks));
            md.push_str(&format!(
                "| **Completion** | {}/{} ({:.0}%) |\n",
                completed_tasks,
                total_tasks,
                (completed_tasks as f32 / total_tasks as f32) * 100.0
            ));
            md.push_str(&format!("| **High Risk** | 🔴 {} |\n", high_risk));
            md.push_str(&format!("| **Medium Risk** | 🟡 {} |\n", medium_risk));
            md.push_str(&format!("| **Low Risk** | 🟢 {} |\n", low_risk));
            md.push_str("\n--- \n\n");
        }

        if options.include_metadata {
            // Metadata
            md.push_str("## Metadata\n\n");
            md.push_str(&format!("- **Review ID:** `{}`\n", data.review.id));
            md.push_str(&format!("- **Run ID:** `{}`\n", data.run.id));
            md.push_str(&format!("- **Agent:** `{}`\n", data.run.agent_id));
            md.push_str(&format!("- **Created At:** {}\n", data.review.created_at));
            md.push_str("\n--- \n\n");
        }

        if options.include_tasks {
            let mut tasks_by_subflow: HashMap<Option<String>, Vec<&ReviewTask>> = HashMap::new();
            for task in &data.tasks {
                tasks_by_subflow
                    .entry(task.sub_flow.clone())
                    .or_default()
                    .push(task);
            }

            let mut subflow_names: Vec<_> = tasks_by_subflow.keys().collect();
            subflow_names.sort_by(|a, b| match (a, b) {
                (Some(a), Some(b)) => a.cmp(b),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            });

            // Table of Contents
            md.push_str("## Table of Contents\n\n");
            for subflow in &subflow_names {
                let name = subflow.as_deref().unwrap_or("Uncategorized");
                md.push_str(&format!(
                    "- [{}](#{})\n",
                    name,
                    name.to_lowercase().replace(" ", "-")
                ));
                if let Some(tasks) = tasks_by_subflow.get(subflow) {
                    for task in tasks {
                        md.push_str(&format!(
                            "  - [{}](#{})\n",
                            task.title,
                            task.title
                                .to_lowercase()
                                .replace(" ", "-")
                                .replace(|c: char| !c.is_alphanumeric() && c != '-', "")
                        ));
                    }
                }
            }
            md.push_str("\n--- \n\n");

            md.push_str("## Details\n\n");
            for subflow in subflow_names {
                let subflow_title = subflow.as_deref().unwrap_or("Uncategorized");
                md.push_str(&format!("### {}\n\n", subflow_title));
                let tasks = tasks_by_subflow.get(subflow).unwrap();
                for task in tasks {
                    let status_icon = match task.status {
                        ReviewStatus::Done => "[x]",
                        ReviewStatus::Ignored => "[~]",
                        _ => "[ ]",
                    };
                    let risk_icon = get_risk_icon(task.stats.risk);
                    md.push_str(&format!(
                        "#### {} {} {}\n\n",
                        status_icon, risk_icon, task.title
                    ));

                    if !task.diff_refs.is_empty() {
                        md.push_str("##### Files Affected\n\n| File | Changes (+/-) | Lines Impacted |\n| :--- | :--- | :--- |\n");
                        for (i, diff_ref) in task.diff_refs.iter().enumerate() {
                            let hunks = diff_ref
                                .hunks
                                .iter()
                                .map(|h| {
                                    format!("L{}-{}", h.old_start + 1, h.old_start + h.old_lines)
                                })
                                .collect::<Vec<_>>()
                                .join(", ");
                            let changes = if i == 0 {
                                format!("+{} / -{}", task.stats.additions, task.stats.deletions)
                            } else {
                                "-".to_string()
                            };
                            md.push_str(&format!(
                                "| `{}` | {} | `{}` |\n",
                                diff_ref.file, changes, hunks
                            ));
                        }
                        md.push('\n');
                    }

                    md.push_str("##### What's Changed\n\n");
                    md.push_str(&format!(
                        "{}\n\n",
                        crate::infra::normalize_newlines(&task.description)
                    ));

                    if let Some(insight) = &task.insight {
                        let insight_clean = crate::infra::normalize_newlines(insight);
                        md.push_str(&format!(
                            "> [!TIP]\n> **AI Insight:** {}\n\n",
                            insight_clean.replace("\n", "\n> ")
                        ));
                    }

                    if let Some(diagram_code) = &task.diagram {
                        md.push_str("##### Diagram\n\n");
                        if let Some(Ok(ascii)) = render_results.get(diagram_code) {
                            md.push_str("```text\n");
                            md.push_str(ascii);
                            md.push_str("\n```\n\n");
                        } else {
                            // Fallback to raw code if rendering failed
                            md.push_str("```d2\n");
                            md.push_str(diagram_code);
                            md.push_str("\n```\n\n");
                        }
                    }

                    let task_feedbacks: Vec<_> = data
                        .feedbacks
                        .iter()
                        .filter(|t| t.task_id.as_ref() == Some(&task.id))
                        .collect();
                    if !task_feedbacks.is_empty() && options.include_feedbacks {
                        md.push_str("##### Feedback\n\n");
                        render_feedbacks_inline(
                            &mut md,
                            &task_feedbacks,
                            &data.comments,
                            &data.tasks,
                            &render_results,
                        );
                    }
                    md.push_str("--- \n\n");
                }
            }
        } else if options.include_feedbacks {
            // -- Selective Mode (Just Feedback) --
            if data.feedbacks.is_empty() {
                md.push_str("_No feedback feedbacks selected._\n");
            } else {
                render_feedbacks_inline(
                    &mut md,
                    &data.feedbacks.iter().collect::<Vec<_>>(),
                    &data.comments,
                    &data.tasks,
                    &render_results,
                );
            }
        }

        Ok(ExportResult {
            markdown: md,
            assets,
        })
    }
}

fn render_feedbacks_inline(
    md: &mut String,
    feedbacks: &[&Feedback],
    all_comments: &[Comment],
    tasks: &[ReviewTask],
    render_results: &HashMap<Arc<str>, Result<String, String>>,
) {
    let mut comments_by_feedback: HashMap<&str, Vec<&Comment>> = HashMap::new();
    for comment in all_comments {
        comments_by_feedback
            .entry(comment.feedback_id.as_str())
            .or_default()
            .push(comment);
    }

    for feedback in feedbacks {
        md.push_str(&format!(
            "### {} [{}] {}\n\n",
            impact_icon(feedback.impact),
            feedback_impact_label(feedback.impact),
            feedback.title
        ));

        if let Some(anchor) = &feedback.anchor
            && let Some(path) = anchor.file_path.as_deref()
            && let Some(line) = anchor.line_number
        {
            md.push_str(&format!("**File:** `{}` (Line {})\n\n", path, line));
        }

        // Add diagram as ASCII if available
        if let Some(task_id) = &feedback.task_id
            && let Some(task) = tasks.iter().find(|t| &t.id == task_id)
            && let Some(diagram_code) = &task.diagram
            && let Some(Ok(ascii)) = render_results.get(diagram_code)
        {
            md.push_str("#### Diagram\n\n```text\n");
            md.push_str(ascii);
            md.push_str("\n```\n\n");
        }

        if let Some(comments) = comments_by_feedback.get(feedback.id.as_str()) {
            for (i, comment) in comments.iter().enumerate() {
                if i == 0 {
                    md.push_str(&format!("{}\n\n", comment.body));
                } else {
                    md.push_str(&format!("> **{}**: {}\n\n", comment.author, comment.body));
                }
            }
        }
    }
}

fn impact_icon(impact: FeedbackImpact) -> &'static str {
    match impact {
        FeedbackImpact::Blocking => "🔴",
        FeedbackImpact::Nitpick => "⚪",
        FeedbackImpact::NiceToHave => "🔵",
    }
}

fn feedback_impact_label(impact: FeedbackImpact) -> &'static str {
    match impact {
        FeedbackImpact::Blocking => "blocking",
        FeedbackImpact::NiceToHave => "nice_to_have",
        FeedbackImpact::Nitpick => "nitpick",
    }
}

fn get_risk_icon(risk: RiskLevel) -> &'static str {
    match risk {
        RiskLevel::Low => "🟢",
        RiskLevel::Medium => "🟡",
        RiskLevel::High => "🔴",
    }
}
