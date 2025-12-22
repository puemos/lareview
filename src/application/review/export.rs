use crate::domain::{
    Comment, Review, ReviewRun, ReviewTask, RiskLevel, TaskStatus, Thread, ThreadImpact,
    ThreadStatus,
};
use crate::infra::d2::d2_to_svg_async;
use anyhow::Result;
use futures::future::join_all;
use std::collections::HashMap;

use std::sync::Arc;

pub struct ExportData {
    pub review: Review,
    pub run: ReviewRun,
    pub tasks: Vec<ReviewTask>,
    pub threads: Vec<Thread>,
    pub comments: Vec<Comment>,
}

#[derive(Debug, Clone)]
pub struct ExportResult {
    pub markdown: String,
    pub assets: std::collections::HashMap<String, Vec<u8>>,
}

fn get_risk_icon(risk: RiskLevel) -> &'static str {
    match risk {
        RiskLevel::Low => "🟢",
        RiskLevel::Medium => "🟡",
        RiskLevel::High => "🔴",
    }
}

pub struct ReviewExporter;

impl ReviewExporter {
    pub async fn export_to_markdown(data: &ExportData, for_preview: bool) -> Result<ExportResult> {
        // Collect all unique diagrams to render in parallel
        let mut diagrams_to_render: Vec<Arc<str>> = Vec::new();
        for task in &data.tasks {
            if let Some(diagram_code) = &task.diagram
                && !diagrams_to_render.contains(diagram_code)
            {
                diagrams_to_render.push(diagram_code.clone());
            }
        }

        // Render all diagrams in parallel
        let render_tasks = diagrams_to_render.iter().map(|code| {
            let code_clone = code.clone();
            async move {
                (
                    code_clone.clone(),
                    d2_to_svg_async(&code_clone, false).await,
                )
            }
        });

        let render_results: HashMap<Arc<str>, Result<String, String>> =
            join_all(render_tasks).await.into_iter().collect();

        let mut md = String::new();
        let mut assets = std::collections::HashMap::new();

        // Title and Summary
        md.push_str(&format!("# {}\n\n", data.review.title));
        if let Some(summary) = &data.review.summary {
            md.push_str(&format!("{}\n\n", summary));
        }

        // Stats Overview
        md.push_str("## Overview\n\n");
        let total_tasks = data.tasks.len();
        let completed_tasks = data
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Done)
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

        md.push_str("| Metric | Value |\n");
        md.push_str("| :--- | :--- |\n");
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
        md.push_str("\n---\n\n");

        // Metadata
        md.push_str("## Metadata\n\n");
        md.push_str(&format!("- **Review ID:** `{}`\n", data.review.id));
        md.push_str(&format!("- **Run ID:** `{}`\n", data.run.id));
        md.push_str(&format!("- **Agent:** `{}`\n", data.run.agent_id));
        md.push_str(&format!("- **Created At:** {}\n", data.review.created_at));
        md.push_str("\n---\n\n");

        let mut tasks_by_subflow: std::collections::HashMap<Option<String>, Vec<&ReviewTask>> =
            std::collections::HashMap::new();
        for task in &data.tasks {
            tasks_by_subflow
                .entry(task.sub_flow.clone())
                .or_default()
                .push(task);
        }

        // Sort subflows for deterministic output
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
        md.push_str("\n---\n\n");

        // Tasks
        md.push_str("## Details\n\n");

        for subflow in subflow_names {
            let subflow_title = subflow.as_deref().unwrap_or("Uncategorized");
            md.push_str(&format!("### {}\n\n", subflow_title));

            let tasks = tasks_by_subflow.get(subflow).unwrap();
            for task in tasks {
                let status_icon = match task.status {
                    TaskStatus::Done => "[x]",
                    TaskStatus::Ignored => "[~]",
                    _ => "[ ]",
                };

                let risk_icon = get_risk_icon(task.stats.risk);
                md.push_str(&format!(
                    "#### {} {} {}\n\n",
                    status_icon, risk_icon, task.title
                ));

                // Files Affected
                if !task.diff_refs.is_empty() {
                    md.push_str("##### Files Affected\n\n");
                    md.push_str("| File | Changes (+/-) | Lines Impacted |\n");
                    md.push_str("| :--- | :--- | :--- |\n");

                    for (i, diff_ref) in task.diff_refs.iter().enumerate() {
                        let hunks = diff_ref
                            .hunks
                            .iter()
                            .map(|h| {
                                format!(
                                    "L{}-{}",
                                    h.new_start,
                                    h.new_start + h.new_lines.saturating_sub(1)
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(", ");

                        // We only have total stats for the task, so we show them for the first file or as a summary
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
                let desc = crate::infra::normalize_newlines(&task.description);
                md.push_str(&format!("{}\n\n", desc));

                if let Some(insight) = &task.insight {
                    let insight_clean = crate::infra::normalize_newlines(insight);
                    md.push_str("> [!TIP]\n");
                    md.push_str(&format!(
                        "> **AI Insight:** {}\n\n",
                        insight_clean.replace("\n", "\n> ")
                    ));
                }

                if let Some(diagram_code) = &task.diagram {
                    md.push_str("##### Diagram\n\n");

                    // Include raw D2 code
                    md.push_str("```d2\n");
                    md.push_str(diagram_code);
                    md.push_str("\n```\n\n");

                    // Then SVG (pre-rendered)
                    if let Some(render_result) = render_results.get(diagram_code) {
                        match render_result {
                            Ok(svg) => {
                                let hash = {
                                    use std::hash::{Hash, Hasher};
                                    let mut s = std::collections::hash_map::DefaultHasher::new();
                                    diagram_code.hash(&mut s);
                                    s.finish()
                                };
                                let filename = format!("diagram_{:x}.svg", hash);

                                if for_preview {
                                    let uri = format!("bytes://{}", filename);
                                    assets.insert(uri.clone(), svg.clone().into_bytes());
                                    md.push_str(&format!("![Diagram]({})\n\n", uri));
                                } else {
                                    let relative_path = format!("assets/{}", filename);
                                    assets.insert(filename, svg.clone().into_bytes());
                                    md.push_str(&format!("![Diagram]({})\n\n", relative_path));
                                }
                            }
                            Err(e) => {
                                md.push_str(&format!(
                                    "> [!WARNING]\n> Could not render SVG diagram: {}\n\n",
                                    e
                                ));
                            }
                        }
                    }
                }

                // Threads for this task
                let task_threads: Vec<_> = data
                    .threads
                    .iter()
                    .filter(|t| t.task_id.as_ref() == Some(&task.id))
                    .collect();
                if !task_threads.is_empty() {
                    let mut comments_by_thread: HashMap<&str, Vec<&Comment>> = HashMap::new();
                    for comment in &data.comments {
                        comments_by_thread
                            .entry(comment.thread_id.as_str())
                            .or_default()
                            .push(comment);
                    }

                    md.push_str("##### Discussion\n\n");
                    for thread in task_threads {
                        md.push_str(&format!(
                            "- [{}][{}] {}\n",
                            thread_status_label(thread.status),
                            thread_impact_label(thread.impact),
                            thread.title
                        ));

                        if let Some(anchor) = &thread.anchor
                            && let Some(path) = anchor.file_path.as_deref()
                            && let Some(line) = anchor.line_number
                        {
                            md.push_str(&format!("  - Anchor: {}:{}\n", path, line));
                        }

                        if let Some(comments) = comments_by_thread.get(thread.id.as_str()) {
                            for comment in comments {
                                md.push_str(&format!(
                                    "  - {} ({}): {}\n",
                                    comment.author, comment.created_at, comment.body
                                ));
                            }
                        }
                    }
                    md.push('\n');
                }
                md.push_str("---\n\n");
            }
        }

        Ok(ExportResult {
            markdown: md,
            assets,
        })
    }
}

fn thread_status_label(status: ThreadStatus) -> &'static str {
    match status {
        ThreadStatus::Todo => "todo",
        ThreadStatus::Wip => "wip",
        ThreadStatus::Done => "done",
        ThreadStatus::Reject => "reject",
    }
}

fn thread_impact_label(impact: ThreadImpact) -> &'static str {
    match impact {
        ThreadImpact::Blocking => "blocking",
        ThreadImpact::NiceToHave => "nice_to_have",
        ThreadImpact::Nitpick => "nitpick",
    }
}
