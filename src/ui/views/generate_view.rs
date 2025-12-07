//! Generate view (egui tabbed version)

use eframe::egui;
use tokio;

use crate::acp::{GenerateTasksInput, generate_tasks_with_acp, list_agent_candidates};
use crate::ui::app::GenTab;
use crate::ui::app::{GenMsg, GenResultPayload, LaReviewApp, SelectedAgent};
use crate::ui::components::diff::render_diff_editor;
use crate::ui::components::theme::AppTheme; // make sure you have the enum in your state module

impl LaReviewApp {
    pub fn ui_generate(&mut self, ui: &mut egui::Ui) {
        let theme = AppTheme::default();
        let mut trigger_generate = false;

        let diff_line_count = if self.state.diff_text.is_empty() {
            0
        } else {
            self.state.diff_text.lines().count()
        };

        ui.vertical(|ui| {
            ui.add_space(8.0);

            // Header
            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new("🔍 Generate review tasks")
                        .size(20.0)
                        .color(theme.text_primary),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Generate button
                    let can_generate =
                        !self.state.diff_text.trim().is_empty() && !self.state.is_generating;

                    let generate_label = if self.state.is_generating {
                        "⏳ Generating..."
                    } else {
                        "✨ Generate tasks"
                    };

                    let generate_button = egui::Button::new(
                        egui::RichText::new(generate_label)
                            .size(15.0)
                            .color(egui::Color32::WHITE),
                    )
                    .fill(if can_generate {
                        theme.accent
                    } else {
                        egui::Color32::from_rgb(60, 60, 65)
                    })
                    .corner_radius(6.0)
                    .min_size(egui::vec2(160.0, 32.0));

                    if ui.add_enabled(can_generate, generate_button).clicked() {
                        trigger_generate = true;
                    }

                    ui.add_space(8.0);

                    // Agent selector chips
                    for agent in [SelectedAgent::Codex, SelectedAgent::Gemini, SelectedAgent::Qwen]
                    {
                        let selected = self.state.selected_agent == agent;
                        let label = format!("{:?}", agent);

                        let text = egui::RichText::new(label).color(if selected {
                            egui::Color32::WHITE
                        } else {
                            theme.text_secondary
                        });

                        let chip = egui::Button::new(text)
                            .fill(if selected {
                                theme.accent
                            } else {
                                egui::Color32::from_rgb(45, 45, 50)
                            })
                            .corner_radius(4.0);

                        if ui.add(chip).clicked() {
                            self.state.selected_agent = agent;
                        }
                    }
                });
            });

            ui.add_space(4.0);

            // Status line
            let status_text = if self.state.is_generating {
                "Analyzing diff with the selected agent..."
            } else if self.state.diff_text.trim().is_empty() {
                "Paste a git diff below to get started."
            } else if self.state.generation_error.is_some() {
                "Last generation failed. See details in the banner below."
            } else {
                "Ready to generate tasks for this diff."
            };

            ui.label(
                egui::RichText::new(status_text)
                    .color(theme.text_secondary)
                    .size(12.0),
            );

            // Error banner below header
            if let Some(err) = &self.state.generation_error {
                ui.add_space(4.0);
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(64, 31, 31))
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("⚠").size(16.0));
                            ui.label(
                                egui::RichText::new(err).color(theme.diff_removed_text),
                            );
                        });
                    });
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            // Tabs
            ui.horizontal(|ui| {
                let tab_label = |text: &str, active: bool| {
                    if active {
                        egui::RichText::new(text).color(theme.accent)
                    } else {
                        egui::RichText::new(text).color(theme.text_secondary)
                    }
                };

                let is_diff = self.state.selected_tab == GenTab::Diff;
                if ui.selectable_label(is_diff, tab_label("Diff", is_diff)).clicked() {
                    self.state.selected_tab = GenTab::Diff;
                }

                let is_agent = self.state.selected_tab == GenTab::Agent;
                let agent_title = if self.state.is_generating {
                    "Agent (running)"
                } else {
                    "Agent"
                };
                if ui
                    .selectable_label(is_agent, tab_label(agent_title, is_agent))
                    .clicked()
                {
                    self.state.selected_tab = GenTab::Agent;
                }
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // Tab content
            match self.state.selected_tab {
                GenTab::Diff => {
                    egui::Frame::NONE
                        .fill(egui::Color32::from_rgb(30, 30, 35))
                        .inner_margin(egui::Margin::symmetric(12, 10))
                        .corner_radius(6.0)
                        .stroke(egui::Stroke::new(
                            1.0,
                            egui::Color32::from_rgb(60, 60, 65),
                        ))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if diff_line_count > 0 {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} lines",
                                            diff_line_count
                                        ))
                                        .color(theme.text_secondary)
                                        .weak(),
                                    );
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if !self.state.diff_text.is_empty()
                                            && ui
                                                .button(
                                                    egui::RichText::new("🗑 Clear")
                                                        .color(theme.diff_removed_text),
                                                )
                                                .clicked()
                                        {
                                            self.state.diff_text.clear();
                                            self.state.generation_error = None;
                                        }
                                    },
                                );
                            });

                            ui.add_space(6.0);

                            if self.state.diff_text.is_empty() {
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.state.diff_text)
                                        .hint_text("Paste your git diff here...")
                                        .desired_rows(18)
                                        .font(egui::TextStyle::Monospace),
                                );
                            } else {
                                render_diff_editor(ui, &self.state.diff_text, "diff");
                            }
                        });
                }

                GenTab::Agent => {
                    egui::Frame::NONE
                        .fill(egui::Color32::from_rgb(30, 30, 35))
                        .inner_margin(egui::Margin::symmetric(12, 10))
                        .corner_radius(6.0)
                        .stroke(egui::Stroke::new(
                            1.0,
                            egui::Color32::from_rgb(60, 60, 65),
                        ))
                        .show(ui, |ui| {
                            let status = if self.state.is_generating {
                                "Running"
                            } else if self.state.generation_error.is_some() {
                                "Error"
                            } else if self.state.agent_logs.is_empty()
                                && self.state.agent_messages.is_empty()
                                && self.state.agent_thoughts.is_empty()
                            {
                                "Idle"
                            } else {
                                "Done"
                            };

                            ui.label(
                                egui::RichText::new(status)
                                    .color(theme.text_secondary)
                                    .size(11.0),
                            );

                            ui.add_space(6.0);

                            egui::ScrollArea::vertical()
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    let has_activity = !self.state.agent_logs.is_empty()
                                        || !self.state.agent_messages.is_empty()
                                        || !self.state.agent_thoughts.is_empty();

                                    if !has_activity && !self.state.is_generating {
                                        ui.label(
                                            egui::RichText::new(
                                                "No agent activity yet. Run generation to see logs.",
                                            )
                                            .color(theme.text_secondary)
                                            .size(12.0),
                                        );
                                        return;
                                    }

                                    if self.state.is_generating && !has_activity {
                                        ui.label(
                                            egui::RichText::new("Waiting for agent output...")
                                                .color(theme.text_secondary)
                                                .size(12.0),
                                        );
                                    }

                                    for log in &self.state.agent_logs {
                                        ui.label(
                                            egui::RichText::new(log)
                                                .color(theme.text_secondary)
                                                .monospace()
                                                .size(12.0),
                                        );
                                    }

                                    for msg in &self.state.agent_messages {
                                        ui.label(
                                            egui::RichText::new(msg)
                                                .color(theme.text_primary)
                                                .size(12.0),
                                        );
                                    }

                                    for thought in &self.state.agent_thoughts {
                                        ui.label(
                                            egui::RichText::new(thought)
                                                .color(theme.accent)
                                                .size(12.0)
                                                .italics(),
                                        );
                                    }
                                });
                        });
                }
            }
        });

        if trigger_generate {
            self.start_generation_async();
        }
    }

    pub fn start_generation_async(&mut self) {
        if self.state.diff_text.trim().is_empty() {
            self.state.generation_error = Some("Please paste a git diff first".into());
            return;
        }

        let pr = self.current_pull_request();
        let diff_text = self.state.diff_text.clone();
        let agent = self.state.selected_agent;

        let (agent_cmd, agent_args, start_log) = match agent {
            SelectedAgent::Codex | SelectedAgent::Gemini | SelectedAgent::Qwen => {
                let agent_id = match agent {
                    SelectedAgent::Codex => "codex",
                    SelectedAgent::Gemini => "gemini",
                    SelectedAgent::Qwen => "qwen",
                };

                let candidates = list_agent_candidates();
                let candidate = candidates.into_iter().find(|c| c.id == agent_id);

                let Some(candidate) = candidate else {
                    self.state.is_generating = false;
                    self.state.generation_error =
                        Some(format!("Agent \"{}\" is not configured.", agent_id));
                    return;
                };

                if !candidate.available {
                    self.state.is_generating = false;
                    self.state.generation_error = Some(format!(
                        "{} is not available on PATH. Install it and restart (command: {} {}).",
                        candidate.label,
                        candidate
                            .command
                            .clone()
                            .unwrap_or_else(|| agent_id.to_string()),
                        candidate.args.join(" ")
                    ));
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

        self.state.is_generating = true;
        self.state.generation_error = None;
        self.state.agent_messages.clear();
        self.state.agent_thoughts.clear();
        self.state.agent_logs = vec![start_log.clone()];

        let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();

        let input = GenerateTasksInput {
            pull_request: pr,
            diff_text,
            agent_command: agent_cmd,
            agent_args,
            progress_tx: Some(progress_tx),
            mcp_server_binary: None,
            timeout_secs: Some(500),
            debug: false,
            fake_tasks: None,
            db_path: None,
        };

        let gen_tx = self.gen_tx.clone();

        tokio::spawn(async move {
            let mut result_fut = std::pin::pin!(generate_tasks_with_acp(input));

            loop {
                tokio::select! {
                    evt = progress_rx.recv() => {
                        if let Some(evt) = evt {
                            let _ = gen_tx.send(GenMsg::Progress(evt)).await;
                        }
                    }
                    res = &mut result_fut => {
                        let msg = match res {
                            Ok(res) => {
                                let mut logs = res.logs;
                                logs.insert(0, start_log.clone());
                                GenMsg::Done(Ok(GenResultPayload {
                                    tasks: res.tasks,
                                    messages: res.messages,
                                    thoughts: res.thoughts,
                                    logs,
                                }))
                            }
                            Err(e) => {
                                GenMsg::Done(Err(format!("Generation failed: {}", e)))
                            }
                        };

                        let _ = gen_tx.send(msg).await;
                        break;
                    }
                }
            }
        });
    }
}
