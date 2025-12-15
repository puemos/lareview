use crate::ui::app::{TimelineContent, TimelineItem};
use catppuccin_egui::MOCHA;
use eframe::egui;
use eframe::egui::collapsing_header::CollapsingState;

use agent_client_protocol::{ContentBlock, SessionUpdate, ToolCallStatus};

// Unused helpers removed

pub(super) fn render_timeline_item(ui: &mut egui::Ui, item: &TimelineItem) {
    ui.set_max_width(ui.available_width());

    match &item.content {
        TimelineContent::LocalLog(line) => {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(line)
                        .color(MOCHA.subtext0)
                        .monospace()
                        .size(11.0),
                )
                .wrap(),
            );
        }
        TimelineContent::Update(update) => {
            render_session_update(ui, update);
        }
    }
}

fn render_session_update(ui: &mut egui::Ui, update: &SessionUpdate) {
    let get_status_style = |status: &ToolCallStatus| -> (egui::Color32, &str) {
        match status {
            ToolCallStatus::Pending => (MOCHA.overlay2, "Pending"),
            ToolCallStatus::InProgress => (MOCHA.yellow, "Running"),
            ToolCallStatus::Completed => (MOCHA.green, "Done"),
            ToolCallStatus::Failed => (MOCHA.red, "Failed"),
            _ => (MOCHA.overlay2, "Unknown"),
        }
    };

    match update {
        SessionUpdate::UserMessageChunk(chunk) => {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("User")
                        .color(MOCHA.mauve)
                        .strong()
                        .size(12.0),
                );
                render_content_chunk(ui, chunk, MOCHA.text, false);
            });
        }
        SessionUpdate::AgentMessageChunk(chunk) => {
            render_content_chunk(ui, chunk, MOCHA.text, false);
        }
        SessionUpdate::AgentThoughtChunk(chunk) => {
            // New Design: No quotes, distinct color
            render_content_chunk(ui, chunk, MOCHA.sky, false);
        }
        SessionUpdate::ToolCall(call) => {
            ui.push_id((&call.tool_call_id, "tool_card"), |ui| {
                egui::Frame::group(ui.style())
                    .fill(MOCHA.mantle)
                    .stroke(egui::Stroke::new(1.0, MOCHA.surface0))
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.set_max_width(ui.available_width());

                        let (status_color, status_label) = get_status_style(&call.status);

                        let id = ui.make_persistent_id("collapsing");
                        let default_open = call.status == ToolCallStatus::InProgress;

                        CollapsingState::load_with_default_open(ui.ctx(), id, default_open)
                            .show_header(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(egui_phosphor::regular::WRENCH)
                                            .color(MOCHA.blue),
                                    );

                                    let full_title = &call.title;
                                    let display_title = if full_title.len() > 50 {
                                        format!("{}...", &full_title[..47])
                                    } else {
                                        full_title.clone()
                                    };

                                    ui.label(
                                        egui::RichText::new(display_title)
                                            .strong()
                                            .color(MOCHA.text),
                                    )
                                    .on_hover_text(full_title);

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(status_label)
                                                    .color(status_color)
                                                    .size(11.0),
                                            );
                                        },
                                    );
                                });
                            })
                            .body(|ui| {
                                ui.add_space(4.0);

                                // Show full title if it was truncated
                                if call.title.len() > 50 {
                                    ui.label(
                                        egui::RichText::new("Command:")
                                            .size(11.0)
                                            .color(MOCHA.overlay1)
                                            .strong(),
                                    );
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&call.title)
                                                .monospace()
                                                .size(11.0)
                                                .color(MOCHA.text),
                                        )
                                        .wrap(),
                                    );
                                    ui.add_space(4.0);
                                }

                                if let Some(input) = &call.raw_input {
                                    render_kv_json(ui, "Input", input);
                                }
                                if let Some(output) = &call.raw_output {
                                    render_kv_json(ui, "Output", output);
                                }
                            });
                    });
            });
        }
        SessionUpdate::ToolCallUpdate(update) => {
            // Compact status update
            let status = update.fields.status.unwrap_or(ToolCallStatus::Pending);
            let (color, label) = get_status_style(&status);
            let title = update.fields.title.as_deref().unwrap_or("Tool");

            ui.horizontal(|ui| {
                let icon = match status {
                    ToolCallStatus::Completed => egui_phosphor::regular::CHECK_CIRCLE,
                    ToolCallStatus::Failed => egui_phosphor::regular::WARNING_CIRCLE,
                    _ => egui_phosphor::regular::GEAR,
                };
                ui.label(egui::RichText::new(icon).color(color).size(12.0));
                ui.label(
                    egui::RichText::new(format!("{} -> {}", title, label))
                        .color(MOCHA.subtext0)
                        .size(12.0),
                );
            });
        }
        SessionUpdate::Plan(plan) => {
            // Collapsed by default
            let id = ui.make_persistent_id(format!("plan_{}", plan.entries.len()));
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
                .show_header(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(egui_phosphor::regular::LIST_CHECKS)
                                .color(MOCHA.mauve),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "Review Plan ({} steps)",
                                plan.entries.len()
                            ))
                            .strong()
                            .color(MOCHA.text),
                        );
                    });
                })
                .body(|ui| {
                    super::plan::render_plan_timeline_item(ui, plan);
                });
        }
        SessionUpdate::AvailableCommandsUpdate(_) => {
            // Minimal system log
            ui.label(
                egui::RichText::new("System: Commands updated")
                    .color(MOCHA.overlay0)
                    .italics()
                    .size(10.0),
            );
        }
        SessionUpdate::CurrentModeUpdate(mode) => {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Mode switch:")
                        .color(MOCHA.overlay0)
                        .size(11.0),
                );
                ui.label(
                    egui::RichText::new(mode.current_mode_id.to_string())
                        .color(MOCHA.overlay1)
                        .background_color(MOCHA.surface0)
                        .monospace()
                        .size(10.0),
                );
            });
        }
        _ => {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(format!("{:?}", update))
                        .color(MOCHA.overlay0)
                        .monospace()
                        .size(10.0),
                )
                .wrap(),
            );
        }
    }
}

fn render_content_chunk(
    ui: &mut egui::Ui,
    chunk: &agent_client_protocol::ContentChunk,
    color: egui::Color32,
    italics: bool,
) {
    match &chunk.content {
        ContentBlock::Text(text) => {
            let mut rt = egui::RichText::new(&text.text).color(color).size(13.0); // Slightly larger
            if italics {
                rt = rt.italics();
            }
            ui.add(egui::Label::new(rt).wrap());
        }
        other => {
            let mut rt = egui::RichText::new(format!("{:?}", other))
                .color(color)
                .monospace()
                .size(11.0);
            if italics {
                rt = rt.italics();
            }
            ui.add(egui::Label::new(rt).wrap());
        }
    }
}

fn render_kv_json(ui: &mut egui::Ui, label: &str, value: &serde_json::Value) {
    ui.label(
        egui::RichText::new(label)
            .size(11.0)
            .color(MOCHA.overlay1)
            .strong(),
    );

    if let serde_json::Value::Object(map) = value {
        egui::Grid::new(ui.id().with(label))
            .num_columns(2)
            .striped(true)
            .min_col_width(60.0)
            .max_col_width(ui.available_width() - 80.0) // Leave room for key column
            .show(ui, |ui| {
                for (k, v) in map {
                    ui.label(
                        egui::RichText::new(k)
                            .monospace()
                            .size(11.0)
                            .color(MOCHA.blue),
                    );

                    let v_str = if v.is_string() {
                        v.as_str().unwrap().to_string()
                    } else {
                        v.to_string()
                    };
                    // Truncate very long values but allow reasonable wrapping
                    let shown_v = if v_str.len() > 500 {
                        format!("{}...", &v_str[..500])
                    } else {
                        v_str
                    };

                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(shown_v)
                                .monospace()
                                .size(11.0)
                                .color(MOCHA.subtext0),
                        )
                        .wrap(),
                    );
                    ui.end_row();
                }
            });
    } else {
        // Fallback for non-objects
        let pretty = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
        ui.add(
            egui::TextEdit::multiline(&mut pretty.as_str())
                .font(egui::FontId::monospace(11.0))
                .code_editor()
                .desired_rows(4)
                .desired_width(ui.available_width())
                .interactive(false),
        );
    }
}
