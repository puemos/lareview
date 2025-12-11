//! Generate view UI for LaReview
//! Handles the task generation interface where users can input diffs
//! and select AI agents to generate review tasks.

use catppuccin_egui::MOCHA;
use eframe::egui;
use tokio;

use crate::acp::{GenerateTasksInput, generate_tasks_with_acp, list_agent_candidates};
use crate::ui::app::{GenMsg, GenResultPayload, LaReviewApp, SelectedAgent};
use crate::ui::components::header::{HeaderAction, header};
use crate::ui::components::selection_chips::selection_chips;
use crate::ui::components::status::error_banner;

impl LaReviewApp {
    pub fn ui_generate(&mut self, ui: &mut egui::Ui) {
        let mut trigger_generate = false;
        let mut trigger_reset = false;

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

            // Error banner
            if let Some(err) = &self.state.generation_error {
                ui.add_space(4.0);
                error_banner(ui, err);
            }

            ui.add_space(10.0);
        });

        // Split pane layout - similar to review_view.rs
        let pane_width_id = ui.id().with("pane_width");
        let available_width = ui.available_width();

        // Get the stored right pane width, default to 300 with some reasonable min/max
        let right_width = ui
            .memory(|mem| mem.data.get_temp::<f32>(pane_width_id))
            .unwrap_or(300.0)
            .clamp(250.0, available_width * 0.5); // Right pane max 50% of available width

        let left_width = available_width - right_width;

        let (left_rect, right_rect) = {
            let available = ui.available_rect_before_wrap();

            let left = egui::Rect::from_min_size(
                available.min,
                egui::vec2(left_width, available.height()),
            );

            let right = egui::Rect::from_min_size(
                egui::pos2(available.min.x + left_width, available.min.y),
                egui::vec2(right_width, available.height()),
            );

            (left, right)
        };

        // Left panel - diff content
        let mut left_ui = ui.new_child(egui::UiBuilder::new().max_rect(left_rect));
        {
            egui::Frame::default()
                .fill(left_ui.style().visuals.window_fill)
                .inner_margin(egui::Margin::same(8))
                .show(&mut left_ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 6.0);

                    // Diff section header with action buttons
                    ui.horizontal(|ui| {
                        ui.heading(
                            egui::RichText::new("GIT DIFF")
                                .size(16.0)
                                .color(MOCHA.text),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Show new button if currently generating
                            if self.state.is_generating
                                && ui
                                    .button(
                                        egui::RichText::new(format!("{} New", egui_phosphor::regular::PLUS))
                                            .color(MOCHA.green)
                                    )
                                    .clicked()
                            {
                                trigger_reset = true;
                            }

                            // Show clear button if diff exists and not generating
                            if !self.state.diff_text.is_empty() && !self.state.is_generating
                                && ui
                                    .button(
                                        egui::RichText::new(format!("{} Clear", egui_phosphor::regular::TRASH_SIMPLE))
                                            .color(MOCHA.red)
                                    )
                                    .clicked()
                            {
                                trigger_reset = true;
                            }
                        });
                    });

                    ui.add_space(4.0);

                    // Show either a text editor for pasting or a formatted diff view
                    if self.state.diff_text.is_empty() {
                        // Show a text editor for pasting (when no diff is present)
                        egui::Frame::new()
                            .fill(MOCHA.crust)
                            .inner_margin(egui::Margin::same(4))
                            .stroke(egui::Stroke::new(1.0, MOCHA.surface0))
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt(ui.id().with("diff_input_scroll"))
                                    .show(ui, |ui| {
                                        let editor =
                                            egui::TextEdit::multiline(&mut self.state.diff_text)
                                                .id_salt(ui.id().with("diff_input_editor"))
                                                .hint_text("Paste your git diff here...\n\nExample:\n\ndiff --git a/src/main.rs b/src/main.rs\nindex abcdef1..1234567 100644\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,3 +1,4 @@\n fn main() {\n     println!(\"Hello, world!\");\n+    println!(\"New line added\");\n }")
                                                .font(egui::TextStyle::Monospace)
                                                .desired_width(f32::INFINITY)
                                                .desired_rows(15);
                                        ui.add(editor);
                                    });
                            });
                    } else {
                        // Show formatted diff when content exists
                        crate::ui::components::diff::render_diff_editor(ui, &self.state.diff_text, "diff");
                    }
                });
        }

        // Resize handle - between left and right panes (full height)
        let resize_id = ui.id().with("resize_handle");
        let resize_rect = egui::Rect::from_min_size(
            egui::pos2(left_rect.max.x - 2.0, left_rect.min.y),
            egui::vec2(1.0, ui.available_rect_before_wrap().height()),
        );

        let resize_response = ui.interact(resize_rect, resize_id, egui::Sense::drag());

        if resize_response.dragged()
            && let Some(pointer_pos) = ui.ctx().pointer_interact_pos()
        {
            let new_right_width = available_width - (pointer_pos.x - left_rect.min.x);
            let clamped_width = new_right_width.clamp(250.0, available_width * 0.5);
            ui.memory_mut(|mem| {
                mem.data.insert_temp(pane_width_id, clamped_width);
            });
        }

        let handle_color = if resize_response.hovered() || resize_response.dragged() {
            ui.style().visuals.widgets.active.bg_fill
        } else {
            ui.style().visuals.widgets.inactive.bg_fill
        };
        ui.painter().rect_filled(resize_rect, 0.0, handle_color);

        if resize_response.hovered() || resize_response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }

        // Right panel - agent information
        let mut right_ui = ui.new_child(egui::UiBuilder::new().max_rect(right_rect));
        {
            egui::Frame::default()
                .fill(right_ui.style().visuals.window_fill)
                .inner_margin(egui::Margin::symmetric(8, 8))
                .show(&mut right_ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 4.0);

                    // Agent section header (like in review_view.rs)
                    ui.heading(egui::RichText::new("AGENT").size(16.0).color(MOCHA.text));
                    ui.add_space(4.0);

                    // Load available agents dynamically from the registry
                    let candidates = crate::acp::list_agent_candidates();
                    let available_agents: Vec<SelectedAgent> = candidates
                        .iter()
                        .filter(|c| c.available) // Only show available agents
                        .map(|c| SelectedAgent::from_str(&c.id))
                        .collect();

                    let agent_labels: Vec<String> = candidates
                        .iter()
                        .filter(|c| c.available)
                        .map(|c| c.id.to_uppercase())
                        .collect();

                    // Agent selection chips using dynamic agents
                    selection_chips(
                        ui,
                        &mut self.state.selected_agent,
                        &available_agents,
                        &agent_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                        "AGENT:",
                    );

                    ui.add_space(8.0);

                    // Status section
                    egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::symmetric(10, 8))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("STATUS")
                                    .size(11.0)
                                    .color(MOCHA.subtext0),
                            );
                            ui.add_space(6.0);
                            ui.separator();

                            // Status message
                            let status_text = if self.state.is_generating {
                                "Analyzing diff with the selected agent..."
                            } else if self.state.diff_text.trim().is_empty() {
                                "Awaiting diff input."
                            } else if self.state.generation_error.is_some() {
                                "Last generation failed. See details below."
                            } else {
                                "Ready to generate tasks."
                            };
                            ui.label(
                                egui::RichText::new(status_text)
                                    .color(MOCHA.subtext1)
                                    .size(12.0),
                            );
                        });

                    ui.add_space(8.0);

                    // Agent activity logs
                    egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::symmetric(10, 8))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("AGENT ACTIVITY")
                                        .size(11.0)
                                        .color(MOCHA.subtext0),
                                );

                                // Show clear logs button if there's content
                                if !self.state.agent_logs.is_empty()
                                    || !self.state.agent_messages.is_empty()
                                    || !self.state.agent_thoughts.is_empty()
                                    || !self.state.plans.is_empty()
                                {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .small_button(
                                                    egui::RichText::new(format!(
                                                        "{} Clear",
                                                        egui_phosphor::regular::X
                                                    ))
                                                    .color(MOCHA.overlay2),
                                                )
                                                .clicked()
                                            {
                                                self.state.agent_logs.clear();
                                                self.state.agent_messages.clear();
                                                self.state.agent_thoughts.clear();
                                                self.state.plans.clear();
                                            }
                                        },
                                    );
                                }
                            });

                            ui.add_space(6.0);
                            ui.separator();

                            egui::ScrollArea::vertical()
                                .id_salt(ui.id().with("agent_activity_scroll"))
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    if !self.state.plans.is_empty() {
                                        ui.label(
                                            egui::RichText::new("AGENT PLAN")
                                                .color(MOCHA.lavender)
                                                .strong(),
                                        );
                                        for plan in &self.state.plans {
                                            for entry in &plan.entries {
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "- {}",
                                                        entry.content
                                                    ))
                                                    .color(MOCHA.teal)
                                                    .size(12.0),
                                                );
                                            }
                                        }
                                        ui.add_space(8.0);
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
                                            egui::RichText::new(msg).color(MOCHA.text).size(12.0),
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
                });
        }

        // Handle actions
        if trigger_reset {
            self.reset_generation_state();
        }

        if trigger_generate {
            self.start_generation_async();
        }
    }

    /// Reset the generation state to start fresh
    fn reset_generation_state(&mut self) {
        self.state.diff_text.clear();
        self.state.generation_error = None;
        self.state.is_generating = false;
        self.state.agent_messages.clear();
        self.state.agent_thoughts.clear();
        self.state.agent_logs.clear();
        self.state.plans.clear();
    }

    pub fn start_generation_async(&mut self) {
        if self.state.diff_text.trim().is_empty() {
            self.state.generation_error = Some("Please paste a git diff first".into());
            return;
        }

        let pr = self.current_pull_request();
        let diff_text = self.state.diff_text.clone();
        let agent = self.state.selected_agent.clone();

        let agent_id = &agent.id;

        let candidates = list_agent_candidates();
        let candidate = candidates.into_iter().find(|c| c.id == agent_id.as_str());

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
        let (agent_cmd, agent_args, start_log) = (command, args, start_log);

        self.state.is_generating = true;
        self.state.generation_error = None;
        self.state.agent_messages.clear();
        self.state.agent_thoughts.clear();
        self.state.plans.clear();
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
