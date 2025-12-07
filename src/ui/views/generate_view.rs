//! Generate view (egui version)

use eframe::egui;
use tokio;

use crate::acp::{GenerateTasksInput, generate_tasks_with_acp, list_agent_candidates};
use crate::ui::app::{GenMsg, GenResultPayload, LaReviewApp, SelectedAgent};
use crate::ui::components::diff::render_diff_editor;
use crate::ui::components::theme::AppTheme;

impl LaReviewApp {
    pub fn ui_generate(&mut self, ui: &mut egui::Ui) {
        let theme = AppTheme::default();

        // Header section
        ui.vertical(|ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new("🔍 Generate Review Tasks")
                        .size(20.0)
                        .color(theme.text_primary),
                );
            });
            ui.label(
                egui::RichText::new(
                    "Paste a git diff to automatically generate review tasks using AI",
                )
                .color(theme.text_secondary),
            );
            ui.add_space(4.0);
        });

        // Error banner
        if let Some(err) = &self.state.generation_error {
            ui.add_space(8.0);
            egui::Frame::NONE
                .fill(egui::Color32::from_rgb(64, 31, 31))
                .inner_margin(egui::Margin::symmetric(12, 8))
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("⚠️").size(16.0));
                        ui.label(egui::RichText::new(err).color(theme.diff_removed_text));
                    });
                });
            ui.add_space(8.0);
        }

        ui.separator();

        // Main content area
        ui.vertical(|ui| {
            // Configuration panel
            egui::Frame::NONE
                .fill(egui::Color32::from_rgb(35, 35, 40))
                .inner_margin(egui::Margin::symmetric(12, 10))
                .corner_radius(6.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("⚙️ Configuration")
                                .strong()
                                .color(theme.text_primary)
                        );
                    });

                    ui.add_space(8.0);

                    // Agent selector
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("AI Agent:")
                                .color(theme.text_primary)
                        );
                        ui.add_space(8.0);

                        for agent in [SelectedAgent::Codex, SelectedAgent::Gemini] {
                            let selected = self.state.selected_agent == agent;
                            let button_text = format!("{:?}", agent);

                            let button = egui::Button::new(
                                egui::RichText::new(&button_text)
                                    .color(if selected {
                                        egui::Color32::WHITE
                                    } else {
                                        theme.text_secondary
                                    })
                            )
                            .fill(if selected {
                                theme.accent
                            } else {
                                egui::Color32::from_rgb(45, 45, 50)
                            })
                            .corner_radius(4.0);

                            if ui.add(button).clicked() {
                                self.state.selected_agent = agent;
                            }
                        }
                    });
                });

            ui.add_space(12.0);

            // Diff input section
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("📝 Git Diff")
                            .strong()
                            .size(15.0)
                            .color(theme.text_primary)
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !self.state.diff_text.is_empty() {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} lines",
                                    self.state.diff_text.lines().count()
                                ))
                                .color(theme.text_secondary)
                                .weak()
                            );
                        }

                        // Note about pasting
                        ui.label(
                            egui::RichText::new("💡 Tip: Focus the text area below and use Ctrl+V (Cmd+V on Mac) to paste")
                                .color(theme.text_secondary)
                                .weak()
                                .size(11.0)
                        );

                        // Clear button
                        if !self.state.diff_text.is_empty() {
                            if ui.button(
                                egui::RichText::new("🗑️ Clear")
                                    .color(theme.diff_removed_text)
                            ).clicked() {
                                self.state.diff_text.clear();
                            }
                        }
                    });
                });

                ui.add_space(4.0);

                // Show diff editor or placeholder
                if self.state.diff_text.is_empty() {
                    // Empty state - show placeholder
                    egui::Frame::NONE
                        .fill(egui::Color32::from_rgb(30, 30, 35))
                        .inner_margin(egui::Margin::symmetric(16, 24))
                        .corner_radius(6.0)
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 65)))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    egui::RichText::new("📄")
                                        .size(48.0)
                                );
                                ui.add_space(8.0);
                                ui.label(
                                    egui::RichText::new("No diff provided")
                                        .size(16.0)
                                        .color(theme.text_primary)
                                );
                                ui.label(
                                    egui::RichText::new("Paste a git diff to get started")
                                        .color(theme.text_secondary)
                                );
                                ui.add_space(16.0);

                                // Help text
                                ui.label(
                                    egui::RichText::new("💡 Tip: Use Ctrl+V (Cmd+V on Mac) to paste in the text area below")
                                        .color(theme.text_secondary)
                                        .weak()
                                        .size(11.0)
                                );

                                ui.add_space(12.0);

                                // Manual input option
                                ui.label(
                                    egui::RichText::new("or type/paste directly:")
                                        .color(theme.text_secondary)
                                        .weak()
                                );
                                ui.add_space(4.0);

                                ui.add(
                                    egui::TextEdit::multiline(&mut self.state.diff_text)
                                        .hint_text("Paste your git diff here...")
                                        .desired_rows(6)
                                        .desired_width(ui.available_width() * 0.8)
                                        .font(egui::TextStyle::Monospace)
                                );
                            });
                        });
                } else {
                    // Show the diff with syntax highlighting
                    render_diff_editor(ui, &mut self.state.diff_text, "diff");
                }
            });

            ui.add_space(16.0);

            // Action buttons
            ui.horizontal(|ui| {
                let can_generate = !self.state.diff_text.trim().is_empty()
                    && !self.state.is_generating;

                let button = egui::Button::new(
                    egui::RichText::new(if self.state.is_generating {
                        "⏳ Generating..."
                    } else {
                        "✨ Generate Tasks"
                    })
                    .size(15.0)
                    .color(egui::Color32::WHITE)
                )
                .fill(if can_generate {
                    theme.accent
                } else {
                    egui::Color32::from_rgb(60, 60, 65)
                })
                .corner_radius(6.0)
                .min_size(egui::vec2(160.0, 36.0));

                if ui.add_enabled(can_generate, button).clicked() {
                    self.start_generation_async();
                }

                if self.state.is_generating {
                    ui.spinner();
                    ui.label(
                        egui::RichText::new("Analyzing diff with AI...")
                            .color(theme.text_secondary)
                    );
                }
            });
        });

        // Agent progress messages
        if !self.state.agent_messages.is_empty() || !self.state.agent_logs.is_empty() {
            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            egui::Frame::NONE
                .fill(egui::Color32::from_rgb(30, 30, 35))
                .inner_margin(egui::Margin::symmetric(12, 10))
                .corner_radius(6.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("📡 Agent Activity")
                                .strong()
                                .color(theme.text_primary),
                        );
                    });

                    ui.add_space(8.0);

                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            // Show logs
                            for log in &self.state.agent_logs {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("•").color(theme.text_secondary));
                                    ui.label(
                                        egui::RichText::new(log)
                                            .color(theme.text_secondary)
                                            .monospace()
                                            .size(12.0),
                                    );
                                });
                            }

                            // Show messages
                            for msg in &self.state.agent_messages {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("💬").size(12.0));
                                    ui.label(
                                        egui::RichText::new(msg)
                                            .color(theme.text_primary)
                                            .size(12.0),
                                    );
                                });
                            }

                            // Show thoughts
                            for thought in &self.state.agent_thoughts {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("💭").size(12.0));
                                    ui.label(
                                        egui::RichText::new(thought)
                                            .color(theme.accent)
                                            .size(12.0)
                                            .italics(),
                                    );
                                });
                            }
                        });
                });
        }
    }

    /// Equivalent of the old gpui `start_generation`, now for egui + tokio
    pub fn start_generation_async(&mut self) {
        if self.state.diff_text.trim().is_empty() {
            self.state.generation_error = Some("Please paste a git diff first".into());
            return;
        }

        // Build input PR from current state
        let pr = self.current_pull_request();

        let diff_text = self.state.diff_text.clone();
        let agent = self.state.selected_agent;

        // Get agent command based on selection (same logic as original)
        let (agent_cmd, agent_args, start_log) = match agent {
            SelectedAgent::Codex | SelectedAgent::Gemini => {
                let agent_id = match agent {
                    SelectedAgent::Codex => "codex",
                    SelectedAgent::Gemini => "gemini",
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

        // Set loading state
        self.state.is_generating = true;
        self.state.generation_error = None;
        self.state.agent_messages.clear();
        self.state.agent_thoughts.clear();
        self.state.agent_logs = vec![start_log.clone()];

        // Progress channel for ACP
        let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();

        // Build GenerateTasksInput exactly as original
        let input = GenerateTasksInput {
            pull_request: pr,
            diff_text,
            agent_command: agent_cmd,
            agent_args,
            progress_tx: Some(progress_tx),
            mcp_server_binary: None, // default
            timeout_secs: Some(500), // default timeout
            debug: false,            // no debug
            fake_tasks: None,        // no fixtures
            db_path: None,           // default DB
        };

        // Sender for messages back into egui loop
        let gen_tx = self.gen_tx.clone();

        tokio::spawn(async move {
            let mut result_fut = std::pin::pin!(generate_tasks_with_acp(input));

            loop {
                tokio::select! {
                    evt = progress_rx.recv() => {
                        if let Some(evt) = evt {
                            let _ = gen_tx.send(GenMsg::Progress(evt)).await;
                        } else {
                            // channel closed
                        }
                    }
                    res = &mut result_fut => {
                        let msg = match res {
                            Ok(res) => {
                                // ACP already persisted tasks to DB; we just reflect state
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
