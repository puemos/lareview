//! Generate view UI for LaReview
//! Handles the task generation interface where users can input diffs
//! and select AI agents to generate review tasks.

use catppuccin_egui::MOCHA;
use eframe::egui;
use tokio;

use crate::acp::{GenerateTasksInput, generate_tasks_with_acp, list_agent_candidates};
use crate::ui::app::{GenMsg, GenResultPayload, GenTab, LaReviewApp, SelectedAgent};
use crate::ui::components::diff::render_diff_editor;
use crate::ui::components::header::{HeaderAction, header};
use crate::ui::components::selection_chips::selection_chips;
use crate::ui::components::status::{error_banner, status_label};
use crate::ui::components::tabs::TabBar;

impl LaReviewApp {
    pub fn ui_generate(&mut self, ui: &mut egui::Ui) {
        let mut trigger_generate = false;

        ui.vertical(|ui| {
            let action_text = if self.state.is_generating {
                format!("{} Generating...", egui_phosphor::regular::HOURGLASS_HIGH)
            } else {
                format!("{} Run", egui_phosphor::regular::PLAY)
            };

            // Header
            header(
                ui,
                "Generate",
                Some(HeaderAction::new(
                    action_text.as_str(),
                    !self.state.diff_text.trim().is_empty() && !self.state.is_generating,
                    MOCHA.mauve,
                    || {
                        trigger_generate = true;
                    },
                )),
            );

            ui.add_space(8.0);

            // Tab bar
            TabBar::new(&mut self.state.selected_tab)
                .add("Diff", GenTab::Diff)
                .add("Agent", GenTab::Agent)
                .show(ui);

            ui.add_space(6.0);
            ui.separator();

            // Agent selection chips
            selection_chips(
                ui,
                &mut self.state.selected_agent,
                &[
                    SelectedAgent::Codex,
                    SelectedAgent::Gemini,
                    SelectedAgent::Qwen,
                ],
                &["CODEX", "GEMINI", "QWEN"],
                "AGENT:",
            );

            ui.add_space(4.0);

            // Status
            let status_text = if self.state.is_generating {
                "Analyzing diff with the selected agent..."
            } else if self.state.diff_text.trim().is_empty() {
                "Awaiting diff input."
            } else if self.state.generation_error.is_some() {
                "Last generation failed. See details below."
            } else {
                "Ready to generate tasks."
            };
            status_label(ui, status_text, MOCHA.subtext1);

            // Error banner
            if let Some(err) = &self.state.generation_error {
                ui.add_space(4.0);
                error_banner(ui, err);
            }

            ui.separator();
            ui.add_space(6.0);

            // Tab content
            match self.state.selected_tab {
                GenTab::Diff => {
                    if self.state.diff_text.is_empty() {
                        // Empty state for Diff tab
                        ui.vertical_centered(|ui| {
                            ui.add_space(60.0);
                            ui.label(egui::RichText::new("🔴").size(40.0).color(MOCHA.blue));
                            ui.add_space(10.0);
                            ui.heading(egui::RichText::new("Paste Git Diff").color(MOCHA.text));
                            ui.label(
                                egui::RichText::new("Generate a diff with a command like:")
                                    .color(MOCHA.subtext1),
                            );
                            ui.code("git diff HEAD~1 > my_diff.txt");
                        });

                        ui.add_space(10.0);

                        // Show a minimal text editor for pasting
                        egui::Frame::new()
                            .fill(MOCHA.crust)
                            .inner_margin(egui::Margin::same(4))
                            .stroke(egui::Stroke::new(1.0, MOCHA.surface0))
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    let editor =
                                        egui::TextEdit::multiline(&mut self.state.diff_text)
                                            .hint_text("Paste your git diff here...")
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(f32::INFINITY)
                                            .desired_rows(10);
                                    ui.add(editor);
                                });
                            });
                    } else {
                        if ui
                            .button(egui::RichText::new("🗑 Clear").color(MOCHA.red))
                            .clicked()
                        {
                            self.state.diff_text.clear();
                            self.state.generation_error = None;
                        }
                        let action = render_diff_editor(ui, &self.state.diff_text, "diff");

                        if matches!(action, crate::ui::components::DiffAction::OpenFullWindow) {
                            self.state.full_diff = Some(crate::ui::app::FullDiffView {
                                title: "Generate diff".to_string(),
                                text: self.state.diff_text.clone(),
                            });
                        }
                    }
                }

                GenTab::Agent => {
                    let has_activity = !self.state.agent_logs.is_empty()
                        || !self.state.agent_messages.is_empty()
                        || !self.state.agent_thoughts.is_empty();

                    if !has_activity && !self.state.is_generating {
                        // Empty state for Agent tab
                        ui.vertical_centered(|ui| {
                            ui.add_space(60.0);
                            ui.label(egui::RichText::new("🔴").size(40.0).color(MOCHA.red));
                            ui.add_space(10.0);
                            ui.heading(egui::RichText::new("Agent Idle").color(MOCHA.text));
                            ui.label(
                                egui::RichText::new(
                                    "The agent is ready and waiting for instructions.",
                                )
                                .color(MOCHA.subtext1),
                            );
                            ui.label("1. Paste a diff in the 'Diff' tab.");
                            ui.label("2. Click the '▶ RUN' button.");
                            ui.label("3. Agent logs and thoughts will appear here.");
                        });
                    } else {
                        // Agent activity logs
                        egui::Frame::new()
                            .fill(MOCHA.crust)
                            .stroke(egui::Stroke::new(1.0, MOCHA.surface0))
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("AGENT ACTIVITY")
                                        .size(11.0)
                                        .color(MOCHA.subtext0),
                                );
                                ui.add_space(6.0);
                                ui.separator();

                                egui::ScrollArea::vertical()
                                    .stick_to_bottom(true)
                                    .show(ui, |ui| {
                                        if self.state.is_generating && !has_activity {
                                            ui.label(
                                                egui::RichText::new("Waiting for agent output...")
                                                    .color(MOCHA.subtext1)
                                                    .size(12.0),
                                            );
                                        }

                                        for log in &self.state.agent_logs {
                                            ui.label(
                                                egui::RichText::new(log)
                                                    .color(MOCHA.subtext0)
                                                    .monospace()
                                                    .size(12.0),
                                            );
                                        }

                                        for msg in &self.state.agent_messages {
                                            ui.label(
                                                egui::RichText::new(msg)
                                                    .color(MOCHA.text)
                                                    .size(12.0),
                                            );
                                        }

                                        for thought in &self.state.agent_thoughts {
                                            ui.label(
                                                egui::RichText::new(thought)
                                                    .color(MOCHA.sky)
                                                    .size(12.0)
                                                    .italics(),
                                            );
                                        }
                                    });
                            });
                    }
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
