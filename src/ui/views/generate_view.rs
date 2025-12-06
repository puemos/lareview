//! Generate view - paste diff and generate tasks with ACP

use gpui::{AsyncApp, Context, Entity, SharedString, Window, div, prelude::*, px};

use crate::acp::{GenerateTasksInput, generate_tasks_with_acp, list_agent_candidates};
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

    fn paste_from_clipboard(&self, cx: &mut Context<Self>) {
        let clipboard = cx
            .read_from_clipboard()
            .and_then(|item| item.text())
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty());

        self.state.update(cx, |s, _| match clipboard {
            Some(text) => {
                s.diff_text = text;
                s.generation_error = None;
            }
            None => {
                s.generation_error = Some("Clipboard is empty or not text".to_string());
            }
        });
    }

    fn clear_diff(&self, cx: &mut Context<Self>) {
        self.state.update(cx, |s, _| {
            s.diff_text.clear();
            s.tasks.clear();
            s.generation_error = None;
        });
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

        // Get agent command based on selection
        let (agent_cmd, agent_args, start_log) = match agent {
            SelectedAgent::Codex | SelectedAgent::Gemini => {
                let agent_id = match agent {
                    SelectedAgent::Codex => "codex",
                    SelectedAgent::Gemini => "gemini",
                };

                let candidates = list_agent_candidates();
                let candidate = candidates.into_iter().find(|c| c.id == agent_id);

                let Some(candidate) = candidate else {
                    self.state.update(cx, |s, _| {
                        s.is_generating = false;
                        s.generation_error =
                            Some(format!("Agent \"{}\" is not configured.", agent_id));
                    });
                    return;
                };

                if !candidate.available {
                    self.state.update(cx, |s, _| {
                        s.is_generating = false;
                        s.generation_error = Some(format!(
                            "{} is not available on PATH. Install it and restart (command: {} {}).",
                            candidate.label,
                            candidate.command.unwrap_or_else(|| agent_id.to_string()),
                            candidate.args.join(" ")
                        ));
                    });
                    return;
                }

                let command = candidate.command.unwrap_or_else(|| agent_id.to_string());
                let args = candidate.args;
                let start_log = format!(
                    "Invoking {} via `{}`",
                    candidate.label,
                    std::iter::once(command.clone())
                        .chain(args.iter().cloned())
                        .collect::<Vec<_>>()
                        .join(" ")
                );
                (command, args, start_log)
            }
        };

        // Set loading state
        let start_log_for_state = start_log.clone();
        self.state.update(cx, |s, _| {
            s.is_generating = true;
            s.generation_error = None;
            s.agent_messages.clear();
            s.agent_thoughts.clear();
            s.agent_logs = vec![start_log_for_state.clone()];
        });

        let input = GenerateTasksInput {
            pull_request: pr,
            files: vec![],
            diff_text: Some(diff_text),
            agent_command: agent_cmd,
            agent_args,
        };

        let state_entity = self.state.clone();
        let start_log = start_log.clone();

        // Spawn async task for ACP generation
        cx.spawn(|_this, cx: &mut AsyncApp| {
            let mut app = cx.clone();

            async move {
                let result = generate_tasks_with_acp(input).await;

                let _ = app.update_entity(&state_entity, |state, _| {
                    state.is_generating = false;
                    match result {
                        Ok(res) => {
                            state.tasks = res.tasks;
                            state.agent_messages = res.messages;
                            state.agent_thoughts = res.thoughts;
                            let mut logs = res.logs;
                            logs.insert(0, start_log.clone());
                            state.agent_logs = logs;
                            if state.tasks.is_empty() {
                                state.generation_error = Some("No tasks generated".to_string());
                            } else {
                                // Auto-navigate to review
                                state.current_view = AppView::Review;
                            }
                        }
                        Err(e) => {
                            state.generation_error = Some(format!("Generation failed: {}", e));
                            state.agent_logs = vec![start_log.clone(), e.to_string()];
                        }
                    }
                });
            }
        })
        .detach();
    }

    fn select_agent(&self, agent: SelectedAgent, cx: &mut Context<Self>) {
        self.state.update(cx, |s, _| {
            s.selected_agent = agent;
        });
    }

    #[allow(dead_code)]
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
        let agent_messages = state.agent_messages.clone();
        let agent_thoughts = state.agent_thoughts.clone();
        let agent_logs = state.agent_logs.clone();
        let has_agent_feedback =
            !agent_messages.is_empty() || !agent_thoughts.is_empty() || !agent_logs.is_empty();
        let show_agent_panel = has_agent_feedback || is_generating;

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
                                    .cursor_text()
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.paste_from_clipboard(cx);
                                    }))
                                    .child(if diff_text.is_empty() {
                                        "Click or use the Paste button to load output from \"git diff\"...".to_string()
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
                                    .flex()
                                    .items_center()
                                    .gap(px(spacing.space_3))
                                    .child(
                                        div()
                                            .id("paste-diff-btn")
                                            .px(px(spacing.space_4))
                                            .py(px(spacing.space_2))
                                            .bg(colors.surface_alt)
                                            .border_1()
                                            .border_color(colors.border_strong)
                                            .cursor_pointer()
                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                this.paste_from_clipboard(cx);
                                            }))
                                            .child("Paste from clipboard"),
                                    )
                                    .child(
                                        div()
                                            .id("clear-diff-btn")
                                            .px(px(spacing.space_4))
                                            .py(px(spacing.space_2))
                                            .bg(colors.surface)
                                            .border_1()
                                            .border_color(colors.border)
                                            .cursor_pointer()
                                            .text_color(colors.text_muted)
                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                this.clear_diff(cx);
                                            }))
                                            .child("Clear"),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(colors.text_muted)
                                            .child("Tip: Run 'git diff main...HEAD | pbcopy', then click Paste."),
                                    ),
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
            // Agent communication panel
            .when(show_agent_panel, |this| {
                this.child(
                    div()
                        .bg(colors.surface)
                        .border_1()
                        .border_color(colors.border_strong)
                        .p(px(spacing.space_5))
                        .flex()
                        .flex_col()
                        .gap(px(spacing.space_4))
                        .child(
                            div()
                                .text_lg()
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(colors.text_strong)
                                .child("Agent communication"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(if is_generating {
                                    colors.warning
                                } else {
                                    colors.success
                                })
                                .child(if is_generating {
                                    "Waiting for agent response..."
                                } else {
                                    "Agent run completed."
                                }),
                        )
                        .child(self.render_agent_feed(
                            "Messages",
                            &agent_messages,
                            colors.text,
                            spacing.space_2,
                        ))
                        .child(self.render_agent_feed(
                            "Thoughts",
                            &agent_thoughts,
                            colors.text_muted,
                            spacing.space_2,
                        ))
                        .child(self.render_agent_feed(
                            "Logs",
                            &agent_logs,
                            colors.danger,
                            spacing.space_2,
                        )),
                )
            })
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

    fn render_agent_feed(
        &self,
        label: &str,
        entries: &[String],
        color: gpui::Hsla,
        gap: f32,
    ) -> impl IntoElement {
        let spacing = theme().spacing;

        div()
            .flex()
            .flex_col()
            .gap(px(gap))
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(color)
                    .child(label.to_string()),
            )
            .child(if entries.is_empty() {
                div()
                    .text_sm()
                    .text_color(theme().colors.text_muted)
                    .child("No entries yet.")
                    .into_any_element()
            } else {
                div()
                    .bg(gpui::hsla(0.0, 0.0, 1.0, 0.5))
                    .border_1()
                    .border_color(theme().colors.border)
                    .rounded_md()
                    .id(SharedString::from(format!("agent-feed-{}", label)))
                    .p(px(spacing.space_3))
                    .flex()
                    .flex_col()
                    .gap(px(spacing.space_2))
                    .max_h(px(320.0))
                    .overflow_scroll()
                    .children(entries.iter().enumerate().map(|(i, entry)| {
                        div()
                            .id(SharedString::from(format!("agent-{}-{}", label, i)))
                            .text_sm()
                            .text_color(color)
                            .child(entry.clone())
                    }))
                    .into_any_element()
            })
            .into_any_element()
    }
}
