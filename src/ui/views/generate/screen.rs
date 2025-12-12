use crate::ui::app::{LaReviewApp, SelectedAgent};
use crate::ui::components::header::{HeaderAction, header};
use crate::ui::components::selection_chips::selection_chips;
use crate::ui::components::status::error_banner;
use catppuccin_egui::MOCHA;
use eframe::egui;

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

            if let Some(err) = &self.state.generation_error {
                ui.add_space(4.0);
                error_banner(ui, err);
            }

            ui.add_space(10.0);
        });

        let pane_width_id = ui.id().with("pane_width");
        let available_width = ui.available_width();

        let right_width = ui
            .memory(|mem| mem.data.get_temp::<f32>(pane_width_id))
            .unwrap_or(300.0)
            .clamp(250.0, available_width * 0.5);

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

        let mut left_ui = ui.new_child(egui::UiBuilder::new().max_rect(left_rect));
        {
            egui::Frame::default()
                .fill(left_ui.style().visuals.window_fill)
                .inner_margin(egui::Margin::same(8))
                .show(&mut left_ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 6.0);

                    ui.horizontal(|ui| {
                        ui.heading(
                            egui::RichText::new("GIT DIFF")
                                .size(16.0)
                                .color(MOCHA.text),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if self.state.is_generating
                                && ui
                                    .button(
                                        egui::RichText::new(format!(
                                            "{} New",
                                            egui_phosphor::regular::PLUS
                                        ))
                                        .color(MOCHA.green),
                                    )
                                    .clicked()
                            {
                                trigger_reset = true;
                            }

                            if !self.state.diff_text.is_empty()
                                && !self.state.is_generating
                                && ui
                                    .button(
                                        egui::RichText::new(format!(
                                            "{} Clear",
                                            egui_phosphor::regular::TRASH_SIMPLE
                                        ))
                                        .color(MOCHA.red),
                                    )
                                    .clicked()
                            {
                                trigger_reset = true;
                            }
                        });
                    });

                    ui.add_space(4.0);

                    if self.state.diff_text.is_empty() {
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
                        crate::ui::components::diff::render_diff_editor(
                            ui,
                            &self.state.diff_text,
                            "diff",
                        );
                    }
                });
        }

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

        let mut right_ui = ui.new_child(egui::UiBuilder::new().max_rect(right_rect));
        {
            egui::Frame::default()
                .fill(right_ui.style().visuals.window_fill)
                .inner_margin(egui::Margin::symmetric(8, 8))
                .show(&mut right_ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 4.0);

                    ui.heading(egui::RichText::new("AGENT").size(16.0).color(MOCHA.text));
                    ui.add_space(4.0);

                    let candidates = crate::infra::acp::list_agent_candidates();
                    let available_agents: Vec<SelectedAgent> = candidates
                        .iter()
                        .filter(|c| c.available)
                        .map(|c| SelectedAgent::from_str(&c.id))
                        .collect();

                    let agent_labels: Vec<String> = candidates
                        .iter()
                        .filter(|c| c.available)
                        .map(|c| c.label.clone())
                        .collect();

                    selection_chips(
                        ui,
                        &mut self.state.selected_agent,
                        &available_agents,
                        &agent_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                        "AGENT:",
                    );

                    ui.add_space(8.0);

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

                    if let Some(plan) = &self.state.latest_plan {
                        super::plan::render_plan_panel(ui, plan);
                        ui.add_space(8.0);
                    }

                    egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::symmetric(10, 8))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("AGENT ACTIVITY")
                                        .size(11.0)
                                        .color(MOCHA.subtext0),
                                );

                                if !self.state.agent_timeline.is_empty() {
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
                                                self.state.reset_agent_timeline();
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
                                    for item in &self.state.agent_timeline {
                                        super::timeline::render_timeline_item(ui, item);
                                    }
                                });
                        });
                });
        }

        if trigger_reset {
            self.reset_generation_state();
        }

        if trigger_generate {
            self.start_generation_async();
        }
    }
}
