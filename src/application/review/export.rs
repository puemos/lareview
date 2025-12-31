use crate::domain::{Comment, Feedback, FeedbackImpact, Review, ReviewRun, ReviewTask, RiskLevel};
use crate::infra::diagram::{DiagramRenderer, MermaidRenderer, parse_json};
use anyhow::Result;
use std::collections::HashMap;

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

        // Generate Markdown
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

            let mut subflow_names: Vec<Option<String>> = tasks_by_subflow.keys().cloned().collect();
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
                let anchor = sanitized_anchor_id(name);
                md.push_str(&format!("- [{}]({})\n", name, anchor));
                if let Some(tasks) = tasks_by_subflow.get(subflow) {
                    for task in tasks {
                        let anchor = sanitized_anchor_id(&task.title);
                        md.push_str(&format!("  - [{}]({})\n", task.title, anchor));
                    }
                }
            }
            md.push_str("\n--- \n\n");

            md.push_str("## Review Tasks\n\n");
            for (i, subflow) in subflow_names.into_iter().enumerate() {
                let subflow_title = subflow.as_deref().unwrap_or("Uncategorized");
                let anchor_id = sanitized_anchor_id(subflow_title).replace("#", "");
                md.push_str(&format!(
                    "## Flow {}: {} <a id=\"{}\"></a>\n\n",
                    i + 1,
                    subflow_title,
                    anchor_id
                ));

                let tasks = tasks_by_subflow.get(&subflow).unwrap();
                for task in tasks {
                    let anchor_id = sanitized_anchor_id(&task.title).replace("#", "");
                    let risk_icon = get_risk_icon(task.stats.risk);
                    let risk_level = match task.stats.risk {
                        RiskLevel::High => "High",
                        RiskLevel::Medium => "Medium",
                        RiskLevel::Low => "Low",
                    };

                    md.push_str(&format!(
                        "### Task: {} <a id=\"{}\"></a>\n\n",
                        task.title, anchor_id
                    ));
                    md.push_str(&format!("**Risk:** {} {}\n\n", risk_icon, risk_level));

                    if !task.diff_refs.is_empty() {
                        for diff_ref in &task.diff_refs {
                            let range = if let Some(first_hunk) = diff_ref.hunks.first() {
                                format!(
                                    "#L{}-L{}",
                                    first_hunk.new_start,
                                    first_hunk.new_start + first_hunk.new_lines
                                )
                            } else {
                                String::new()
                            };

                            let link = if let crate::domain::ReviewSource::GitHubPr {
                                owner,
                                repo,
                                head_sha: Some(sha),
                                ..
                            } = &data.review.source
                            {
                                format!(
                                    "https://github.com/{}/{}/blob/{}/{}{}",
                                    owner, repo, sha, diff_ref.file, range
                                )
                            } else {
                                format!("{}{}", diff_ref.file, range)
                            };

                            md.push_str(&format!("- [{}]({})\n", diff_ref.file, link));
                        }
                        md.push('\n');
                    }

                    md.push_str("**What's Changed**\n\n");
                    md.push_str(&format!(
                        "{}\n\n",
                        crate::infra::normalize_newlines(&task.description)
                    ));

                    if let Some(insight) = &task.insight {
                        let insight_clean = crate::infra::normalize_newlines(insight);
                        md.push_str(&format!(
                            "> [!TIP]\n> **Insight:** {}\n\n",
                            insight_clean.replace("\n", "\n> ")
                        ));
                    }

                    if let Some(diagram_code) = &task.diagram {
                        md.push_str("**Diagram**\n\n");
                        match parse_json(diagram_code) {
                            Ok(diagram) => match MermaidRenderer.render(&diagram) {
                                Ok(mermaid) => {
                                    md.push_str("```mermaid\n");
                                    md.push_str(&mermaid);
                                    md.push_str("\n```\n\n");
                                }
                                Err(e) => {
                                    md.push_str(&format!("_Failed to render diagram: {}_\n\n", e));
                                }
                            },
                            Err(e) => {
                                md.push_str(&format!("_Invalid diagram JSON: {}_\n\n", e));
                            }
                        }
                    }

                    let task_feedbacks: Vec<_> = data
                        .feedbacks
                        .iter()
                        .filter(|t| t.task_id.as_ref() == Some(&task.id))
                        .collect();
                    if !task_feedbacks.is_empty() && options.include_feedbacks {
                        md.push_str("**Feedback**\n\n");
                        for feedback in task_feedbacks {
                            let f_comments: Vec<_> = data
                                .comments
                                .iter()
                                .filter(|c| c.feedback_id == feedback.id)
                                .cloned()
                                .collect();
                            md.push_str(&Self::render_single_feedback_markdown(
                                feedback,
                                &f_comments,
                            ));
                        }
                    }
                    md.push_str("--- \n\n");
                }
            }
        } else if options.include_feedbacks {
            // -- Selective Mode (Just Feedback) --
            if data.feedbacks.is_empty() {
                md.push_str("_No feedbacks selected._\n");
            } else {
                for feedback in &data.feedbacks {
                    let f_comments: Vec<_> = data
                        .comments
                        .iter()
                        .filter(|c| c.feedback_id == feedback.id)
                        .cloned()
                        .collect();
                    md.push_str(&Self::render_single_feedback_markdown(
                        feedback,
                        &f_comments,
                    ));
                }
            }
        }

        Ok(ExportResult {
            markdown: md,
            assets,
        })
    }

    /// Renders a single feedback item as markdown, suitable for a GitHub comment.
    pub fn render_single_feedback_markdown(feedback: &Feedback, comments: &[Comment]) -> String {
        let mut md = String::new();
        let emoji = impact_icon(feedback.impact);
        let severity = feedback_impact_label(feedback.impact); // "blocking", "nitpick", etc.
        // Capitalize severity
        let severity = severity
            .chars()
            .next()
            .map(|c| c.to_uppercase().collect::<String>() + &severity[1..])
            .unwrap_or(severity.to_string());

        md.push_str(&format!(
            "**Feedback:** {}\n**Severity:** {} {}\n\n",
            feedback.title, emoji, severity
        ));

        if comments.is_empty() {
            md.push_str("No comments provided.\n");
        } else {
            for comment in comments {
                let author = if let Some(stripped) = comment.author.strip_prefix("agent:") {
                    let mut s = String::from("Agent ");
                    // Capitalize first letter of agent name
                    if let Some(first) = stripped.chars().next() {
                        s.push(first.to_ascii_uppercase());
                        s.push_str(&stripped[1..]);
                    } else {
                        s.push_str(stripped);
                    }
                    s
                } else {
                    comment.author.clone()
                };

                md.push_str(&format!("**{}:**\n{}\n\n", author, comment.body));
            }
        }
        md
    }
}

pub fn sanitized_anchor_id(text: &str) -> String {
    let id = text
        .to_lowercase()
        .replace(" ", "-")
        .replace(|c: char| !c.is_alphanumeric() && c != '-', "");
    format!("#user-content-{}", id)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Review, ReviewRun, ReviewSource};

    #[tokio::test]
    async fn test_export_with_zero_tasks() {
        let now = chrono::Utc::now().to_rfc3339();
        let export_data = ExportData {
            review: Review {
                id: "test-review".into(),
                title: "Test Review".into(),
                summary: Some("Summary".into()),
                source: ReviewSource::DiffPaste {
                    diff_hash: "hash".into(),
                },
                active_run_id: Some("run-1".into()),
                created_at: now.clone(),
                updated_at: now.clone(),
            },
            run: ReviewRun {
                id: "run-1".into(),
                review_id: "test-review".into(),
                agent_id: "agent-1".into(),
                input_ref: "ref".into(),
                diff_text: "".into(),
                diff_hash: "hash".into(),
                created_at: now,
            },
            tasks: vec![],
            feedbacks: vec![],
            comments: vec![],
        };

        let result =
            ReviewExporter::export_to_markdown(&export_data, &ExportOptions::default()).await;
        assert!(result.is_ok());
        let md = result.unwrap().markdown;
        assert!(md.contains("**Total Tasks** | 0 |"));
        // Completion removed
        assert!(!md.contains("**Completion**"));
    }
}
