//! Generate view - paste diff and generate tasks with ACP

use gpui::{div, prelude::*, px, Context, Entity, SharedString, Window};
use gpui::app::AsyncAppContext;

use crate::acp::{generate_tasks_with_acp, GenerateTasksInput};
use crate::domain::PullRequest;
use crate::ui::app::{AppState, AppView, SelectedAgent};
use crate::ui::theme::theme;

/// Generate view for creating review tasks from git diff
pub struct GenerateView {
    state: Entity<AppState>,
}

impl GenerateView {
    pub fn new(state: Entity<AppState>, _cx: &mut Context<impl Render>) -> Self {
        Self { state }
    }

    fn start_generation(&self, cx: &mut Context<Self>) {
        // Read current state
        let state = self.state.read(cx);
        
        if state.diff_text.trim().is_empty() {
            self.state.update(cx, |s, _| {
                s.generation_error = Some("Please paste a git diff first".to_string());
            });
            return;
        }

        // Build input
        let pr = PullRequest {
            id: state.pr_id.clone(),
            title: state.pr_title.clone(),
            repo: state.pr_repo.clone(),
            author: state.pr_author.clone(),
            branch: state.pr_branch.clone(),
            description: None,
            created_at: String::new(),
        };

        let diff_text = state.diff_text.clone();
        let agent = state.selected_agent.clone();

        // Set loading state
        self.state.update(cx, |s, _| {
            s.is_generating = true;
            s.generation_error = None;
        });

        // Get agent command based on selection
        let (agent_cmd, agent_args) = match agent {
            SelectedAgent::Stub => {
                // Use built-in stub - generate simple tasks from diff
                self.generate_stub_tasks(cx, pr, diff_text);
                return;
            }
            SelectedAgent::Codex => ("codex".to_string(), vec!["--acp".to_string()]),
            SelectedAgent::Gemini => ("gemini".to_string(), vec!["--acp".to_string()]),
        };

        let input = GenerateTasksInput {
            pull_request: pr,
            files: vec![],
            diff_text: Some(diff_text),
            agent_command: agent_cmd,
            agent_args,
        };

        let state_entity = self.state.clone();

        // Spawn async task for ACP generation
        cx.spawn(|_this, mut cx: &mut AsyncAppContext| async move {
            let result = generate_tasks_with_acp(input).await;

            let _ = cx.update_entity(&state_entity, |state, _| {
                state.is_generating = false;
                match result {
                    Ok(res) => {
                        state.tasks = res.tasks;
                        if state.tasks.is_empty() {
                            state.generation_error = Some("No tasks generated".to_string());
                        } else {
                            // Auto-navigate to review
                            state.current_view = AppView::Review;
                        }
                    }
                    Err(e) => {
                        state.generation_error = Some(format!("Generation failed: {}", e));
                    }
                }
            });
        })
        .detach();
    }

    fn generate_stub_tasks(&self, cx: &mut Context<Self>, pr: PullRequest, diff_text: String) {
        use crate::acp::parse_diff;
        use crate::domain::{Patch, ReviewTask, RiskLevel, TaskStats};

        let files = parse_diff(&diff_text);
        
        let tasks: Vec<ReviewTask> = files
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let risk = if f.additions > 50 || f.deletions > 50 {
                    RiskLevel::High
                } else if f.additions > 20 || f.deletions > 20 {
                    RiskLevel::Medium
                } else {
                    RiskLevel::Low
                };

                ReviewTask {
                    id: format!("task-{}", i + 1),
                    title: format!("Review changes in {}", f.file_path),
                    description: format!(
                        "Review {} additions and {} deletions in {}",
                        f.additions, f.deletions, f.file_path
                    ),
                    files: vec![f.file_path.clone()],
                    stats: TaskStats {
                        additions: f.additions,
                        deletions: f.deletions,
                        risk,
                        tags: vec![],
                    },
                    patches: vec![Patch {
                        file: f.file_path.clone(),
                        hunk: f.patch.clone(),
                    }],
                    insight: None,
                    diagram: None,
                    ai_generated: false,
                }
            })
            .collect();

        self.state.update(cx, |state, _| {
            state.is_generating = false;
            state.tasks = tasks;
            state.pr = Some(pr);
            if state.tasks.is_empty() {
                state.generation_error = Some("No files found in diff".to_string());
            } else {
                state.current_view = AppView::Review;
            }
        });
    }

    fn select_agent(&self, agent: SelectedAgent, cx: &mut Context<Self>) {
        self.state.update(cx, |s, _| {
            s.selected_agent = agent;
        });
    }

    fn update_diff(&self, text: String, cx: &mut Context<Self>) {
        self.state.update(cx, |s, _| {
            s.diff_text = text;
        });
    }
}

impl Render for GenerateView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = theme().colors;
        let spacing = theme().spacing;
        let state = self.state.read(cx);

        let is_generating = state.is_generating;
        let selected_agent = state.selected_agent.clone();
        let diff_text = state.diff_text.clone();
        let generation_error = state.generation_error.clone();

        div()
            .flex()
            .flex_col()
            .gap(px(spacing.space_6))
            .max_w(px(960.0))
            .mx_auto()
            // Header
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(spacing.space_2))
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(colors.text_strong)
                            .child("Generate review tasks from a git diff"),
                    )
                    .child(
                        div()
                            .text_color(colors.text_muted)
                            .child("Paste a unified git diff. We'll analyze it and create review tasks."),
                    ),
            )
            // Error message
            .when_some(generation_error, |this, err| {
                this.child(
                    div()
                        .bg(colors.danger)
                        .text_color(colors.surface)
                        .p(px(spacing.space_3))
                        .child(err),
                )
            })
            // Form panel
            .child(
                div()
                    .bg(colors.surface)
                    .border_1()
                    .border_color(colors.border_strong)
                    .p(px(spacing.space_6))
                    .flex()
                    .flex_col()
                    .gap(px(spacing.space_5))
                    // Diff textarea (simplified - shows current content)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(spacing.space_2))
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("Git diff"),
                            )
                            .child(
                                div()
                                    .id("diff-input")
                                    .bg(colors.surface_alt)
                                    .border_1()
                                    .border_color(colors.border_strong)
                                    .p(px(spacing.space_3))
                                    .min_h(px(200.0))
                                    .font_family("JetBrains Mono")
                                    .text_sm()
                                    .text_color(if diff_text.is_empty() {
                                        colors.text_muted
                                    } else {
                                        colors.text
                                    })
                                    .child(if diff_text.is_empty() {
                                        "Paste output from \"git diff\" here...".to_string()
                                    } else {
                                        let preview = diff_text.lines().take(10).collect::<Vec<_>>().join("\n");
                                        if diff_text.lines().count() > 10 {
                                            format!("{}\\n... ({} more lines)", preview, diff_text.lines().count() - 10)
                                        } else {
                                            preview
                                        }
                                    }),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(colors.text_muted)
                                    .child("Tip: Run 'git diff main...HEAD | pbcopy' then paste here"),
                            ),
                    )
                    // Agent selection
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(spacing.space_3))
                            .border_1()
                            .border_color(colors.border)
                            .p(px(spacing.space_4))
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("Agent"),
                            )
                            .child(self.render_agent_option(
                                "Built-in stub",
                                "Available",
                                true,
                                SelectedAgent::Stub,
                                selected_agent == SelectedAgent::Stub,
                                cx,
                            ))
                            .child(self.render_agent_option(
                                "Codex (ACP)",
                                "Requires codex CLI",
                                false,
                                SelectedAgent::Codex,
                                selected_agent == SelectedAgent::Codex,
                                cx,
                            ))
                            .child(self.render_agent_option(
                                "Gemini (ACP)",
                                "Requires gemini CLI",
                                false,
                                SelectedAgent::Gemini,
                                selected_agent == SelectedAgent::Gemini,
                                cx,
                            )),
                    )
                    // Submit button
                    .child(
                        div()
                            .id("generate-btn")
                            .bg(if is_generating {
                                colors.text_muted
                            } else {
                                colors.text_strong
                            })
                            .text_color(colors.surface)
                            .px(px(spacing.space_6))
                            .py(px(spacing.space_3))
                            .border_1()
                            .border_color(colors.text_strong)
                            .cursor_pointer()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                this.start_generation(cx);
                            }))
                            .child(if is_generating {
                                "Generating..."
                            } else {
                                "Generate tasks"
                            }),
                    ),
            )
    }
}

impl GenerateView {
    fn render_agent_option(
        &self,
        label: &str,
        status: &str,
        available: bool,
        agent: SelectedAgent,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = theme().colors;
        let spacing = theme().spacing;
        let agent_clone = agent.clone();

        div()
            .id(SharedString::from(format!("agent-{:?}", agent)))
            .flex()
            .items_center()
            .gap(px(spacing.space_3))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _event, _window, cx| {
                this.select_agent(agent_clone.clone(), cx);
            }))
            .child(
                div()
                    .size(px(16.0))
                    .border_1()
                    .border_color(colors.border_strong)
                    .bg(if selected {
                        colors.primary
                    } else {
                        colors.surface
                    }),
            )
            .child(
                div()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(label.to_string()),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(if available {
                        colors.success
                    } else {
                        colors.text_muted
                    })
                    .child(status.to_string()),
            )
    }
}
