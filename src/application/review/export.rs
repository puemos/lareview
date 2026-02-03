use crate::domain::{
    Comment, Feedback, FeedbackImpact, FeedbackSide, MergeConfidence, Review, ReviewRun,
    ReviewTask, RiskLevel,
};
use crate::infra::diff::index::DiffIndex;
use anyhow::Result;
use std::collections::HashSet;

pub struct ExportData {
    pub review: Review,
    pub run: ReviewRun,
    pub tasks: Vec<ReviewTask>,
    pub feedbacks: Vec<Feedback>,
    pub comments: Vec<Comment>,
    pub merge_confidence: Option<MergeConfidence>,
}

#[derive(Debug, Clone)]
pub struct ExportResult {
    pub markdown: String,
    pub assets: std::collections::HashMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExportOptions {
    pub include_summary: bool,
    pub include_stats: bool,
    pub include_metadata: bool,
    pub include_tasks: bool,
    pub include_feedbacks: bool,
    pub include_context_diff: bool,
    pub include_toc: bool,
    pub selected_tasks: Option<HashSet<String>>,
    pub selected_feedbacks: Option<HashSet<String>>,
}

pub struct ReviewExporter;

impl ReviewExporter {
    pub async fn export_to_markdown(
        data: &ExportData,
        options: &ExportOptions,
    ) -> Result<ExportResult> {
        let mut md = String::new();
        let assets = std::collections::HashMap::new();
        let diff_index = DiffIndex::new(&data.run.diff_text).ok();

        // Title
        md.push_str(&format!("# {}\n\n", data.review.title));

        if options.include_toc {
            md.push_str("## Table of Contents\n\n");
            md.push_str("- [Overview](#overview)\n");
            md.push_str("- [Metadata](#metadata)\n");

            if options.include_tasks {
                md.push_str("- [Tasks](#tasks)\n");
                for task in &data.tasks {
                    if let Some(selected) = &options.selected_tasks
                        && !selected.contains(&task.id)
                    {
                        continue;
                    }
                    let slug = Self::slugify(&task.title);
                    md.push_str(&format!("  - [{}](#{})\n", task.title, slug));
                }
            } else if options.include_feedbacks {
                md.push_str("- [Feedback](#feedback)\n");
            }
            md.push_str("\n---\n\n");
        }

        if options.include_summary
            && let Some(summary) = &data.review.summary
        {
            md.push_str(&format!("{}\n\n", summary));
        }

        if options.include_stats {
            md.push_str("## Overview\n\n");

            // Merge Confidence
            if let Some(confidence) = &data.merge_confidence {
                md.push_str(&format!(
                    "### Merge Confidence: {:.1}/5 - {}\n\n",
                    confidence.score,
                    confidence.label()
                ));
                md.push_str(&format!("**\"{}\"**\n\n", confidence.recommendation()));

                if !confidence.reasons.is_empty() {
                    md.push_str("**Assessment:**\n");
                    for reason in &confidence.reasons {
                        md.push_str(&format!("- {}\n", reason));
                    }
                    md.push('\n');
                }

                md.push_str("---\n\n");
            }

            let total_tasks = data.tasks.len();
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
            md.push_str(&format!("| **High Risk** | 🔴 {} |\n", high_risk));
            md.push_str(&format!("| **Medium Risk** | 🟡 {} |\n", medium_risk));
            md.push_str(&format!("| **Low Risk** | 🟢 {} |\n", low_risk));
            md.push_str("\n--- \n\n");
        }

        if options.include_metadata {
            md.push_str("## Metadata\n\n");
            md.push_str(&format!("- **Review ID:** `{}`\n", data.review.id));
            md.push_str(&format!("- **Run ID:** `{}`\n", data.run.id));
            md.push_str(&format!("- **Agent:** `{}`\n", data.run.agent_id));
            md.push_str(&format!("- **Created At:** {}\n", data.review.created_at));
            md.push_str("\n--- \n\n");
        }

        let mut rendered_feedback_ids = HashSet::new();
        if options.include_tasks {
            let mut rendered_tasks_header = false;

            for task in &data.tasks {
                if let Some(selected) = &options.selected_tasks
                    && !selected.contains(&task.id)
                {
                    continue;
                }

                if !rendered_tasks_header {
                    md.push_str("## Tasks\n\n");
                    rendered_tasks_header = true;
                }

                md.push_str(&format!("### {}\n\n", task.title));
                md.push_str(&format!("**Risk:** {}\n\n", task.stats.risk));
                md.push_str(&format!("{}\n\n", task.description));

                if let Some(insight) = &task.insight {
                    md.push_str(&format!(
                        "> [!TIP]\n> **Insight:** {}\n\n",
                        insight.replace("\n", "\n> ")
                    ));
                }

                if let Some(diagram) = &task.diagram {
                    md.push_str("**Diagram:**\n\n```mermaid\n");
                    md.push_str(diagram);
                    md.push_str("\n```\n\n");
                }

                let task_feedbacks: Vec<_> = data
                    .feedbacks
                    .iter()
                    .filter(|f| f.task_id.as_ref() == Some(&task.id))
                    .collect();

                if !task_feedbacks.is_empty() && options.include_feedbacks {
                    let mut rendered_feedback_for_task = false;
                    for feedback in task_feedbacks {
                        if let Some(selected) = &options.selected_feedbacks
                            && !selected.contains(&feedback.id)
                        {
                            continue;
                        }

                        if !rendered_feedback_for_task {
                            md.push_str("**Feedback for this task:**\n\n");
                            rendered_feedback_for_task = true;
                        }

                        let comments: Vec<_> = data
                            .comments
                            .iter()
                            .filter(|c| c.feedback_id == feedback.id)
                            .cloned()
                            .collect();

                        let diff_snippet = if options.include_context_diff {
                            if let (Some(index), Some(anchor)) = (&diff_index, &feedback.anchor) {
                                if let (Some(path), Some(line)) =
                                    (&anchor.file_path, anchor.line_number)
                                {
                                    index
                                        .find_hunk_at_line(
                                            path,
                                            line,
                                            anchor.side.unwrap_or(FeedbackSide::New),
                                        )
                                        .map(|indexed| {
                                            DiffIndex::render_hunk_unified(
                                                &indexed.hunk,
                                                indexed.coords,
                                            )
                                        })
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        md.push_str(&Self::render_single_feedback_markdown(
                            feedback,
                            &comments,
                            diff_snippet.as_deref(),
                        ));
                        rendered_feedback_ids.insert(feedback.id.clone());
                    }
                }
                md.push_str("--- \n\n");
            }
        }

        if options.include_feedbacks {
            let mut rendered_feedback_header = false;
            for feedback in &data.feedbacks {
                if rendered_feedback_ids.contains(&feedback.id) {
                    continue;
                }
                if let Some(selected) = &options.selected_feedbacks
                    && !selected.contains(&feedback.id)
                {
                    continue;
                }

                if !rendered_feedback_header {
                    md.push_str("## Feedback\n\n");
                    rendered_feedback_header = true;
                }

                let comments: Vec<_> = data
                    .comments
                    .iter()
                    .filter(|c| c.feedback_id == feedback.id)
                    .cloned()
                    .collect();

                let diff_snippet = if options.include_context_diff {
                    if let (Some(index), Some(anchor)) = (&diff_index, &feedback.anchor) {
                        if let (Some(path), Some(line)) = (&anchor.file_path, anchor.line_number) {
                            index
                                .find_hunk_at_line(
                                    path,
                                    line,
                                    anchor.side.unwrap_or(FeedbackSide::New),
                                )
                                .map(|indexed| {
                                    DiffIndex::render_hunk_unified(&indexed.hunk, indexed.coords)
                                })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                md.push_str(&Self::render_single_feedback_markdown(
                    feedback,
                    &comments,
                    diff_snippet.as_deref(),
                ));
            }
        }

        Ok(ExportResult {
            markdown: md,
            assets,
        })
    }

    pub fn render_task_markdown(task: &ReviewTask) -> String {
        let mut md = String::new();
        md.push_str(&format!("### {}\n\n", task.title));
        md.push_str(&format!("**Risk:** {}\n\n", task.stats.risk));
        md.push_str(&format!("{}\n\n", task.description));

        if let Some(insight) = &task.insight {
            md.push_str(&format!(
                "> [!TIP]\n> **Insight:** {}\n\n",
                insight.replace("\n", "\n> ")
            ));
        }

        if let Some(diagram) = &task.diagram {
            md.push_str("**Diagram:**\n\n```mermaid\n");
            md.push_str(diagram);
            md.push_str("\n```\n\n");
        }
        md
    }

    pub fn render_single_feedback_markdown(
        feedback: &Feedback,
        comments: &[Comment],
        diff_snippet: Option<&str>,
    ) -> String {
        let mut md = String::new();
        let emoji = match feedback.impact {
            FeedbackImpact::Blocking => "🔴",
            FeedbackImpact::Nitpick => "⚪",
            FeedbackImpact::NiceToHave => "🔵",
        };
        let severity = match feedback.impact {
            FeedbackImpact::Blocking => "Blocking",
            FeedbackImpact::NiceToHave => "Nice to Have",
            FeedbackImpact::Nitpick => "Nitpick",
        };

        md.push_str(&format!(
            "**Feedback:** {}<br>\n**Severity:** {} {}\n\n",
            feedback.title, emoji, severity
        ));

        if let Some(snippet) = diff_snippet {
            md.push_str("**Context:**\n\n```diff\n");
            md.push_str(snippet);
            md.push_str("\n```\n\n");
        }

        if comments.is_empty() {
            md.push_str("No comments provided.\n\n");
        } else {
            for comment in comments {
                let author = if let Some(stripped) = comment.author.strip_prefix("agent:") {
                    format!("Agent {}{}", &stripped[0..1].to_uppercase(), &stripped[1..])
                } else {
                    comment.author.clone()
                };

                md.push_str(&format!("**{}:**\n{}\n\n", author, comment.body));
            }
        }
        md
    }

    fn slugify(text: &str) -> String {
        text.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }
}
