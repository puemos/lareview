use crate::ui::app::{TimelineContent, TimelineItem};
use crate::ui::spacing;
use catppuccin_egui::MOCHA;
use eframe::egui;

use agent_client_protocol::{ContentBlock, SessionUpdate, ToolCallStatus};

pub(super) fn render_timeline_item(ui: &mut egui::Ui, item: &TimelineItem) {
    match &item.content {
        TimelineContent::LocalLog(line) => {
            ui.label(
                egui::RichText::new(line)
                    .color(MOCHA.subtext0)
                    .monospace()
                    .size(12.0),
            );
        }
        TimelineContent::Update(update) => {
            render_session_update(ui, update);
        }
    }
}

fn render_session_update(ui: &mut egui::Ui, update: &SessionUpdate) {
    match update {
        SessionUpdate::UserMessageChunk(chunk) | SessionUpdate::AgentMessageChunk(chunk) => {
            render_content_chunk(ui, chunk, MOCHA.text, false);
        }
        SessionUpdate::AgentThoughtChunk(chunk) => {
            render_content_chunk(ui, chunk, MOCHA.sky, true);
        }
        SessionUpdate::ToolCall(call) => {
            let status_color = match call.status {
                ToolCallStatus::Pending => MOCHA.overlay2,
                ToolCallStatus::InProgress => MOCHA.yellow,
                ToolCallStatus::Completed => MOCHA.green,
                ToolCallStatus::Failed => MOCHA.red,
                _ => MOCHA.overlay2,
            };
            let status_label = match call.status {
                ToolCallStatus::Pending => "pending",
                ToolCallStatus::InProgress => "in progress",
                ToolCallStatus::Completed => "completed",
                ToolCallStatus::Failed => "failed",
                _ => "",
            };
            let header_text = egui::RichText::new(format!("{} ({})", call.title, status_label))
                .color(status_color)
                .size(12.0);

            egui::CollapsingHeader::new(header_text)
                .id_salt(("tool_call", call.tool_call_id.clone()))
                .default_open(matches!(
                    call.status,
                    ToolCallStatus::Pending | ToolCallStatus::InProgress
                ))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing =
                        egui::vec2(spacing::TIGHT_ITEM_SPACING.0, spacing::TIGHT_ITEM_SPACING.1); // 4.0, 4.0

                    if let Some(kind) =
                        (!matches!(call.kind, agent_client_protocol::ToolKind::Other))
                            .then_some(call.kind)
                    {
                        ui.label(
                            egui::RichText::new(format!("kind: {kind:?}"))
                                .color(MOCHA.subtext0)
                                .size(11.0),
                        );
                    }

                    if let Some(input) = &call.raw_input {
                        render_json_block(
                            ui,
                            ("tool_json", call.tool_call_id.clone(), "input"),
                            "input",
                            input,
                        );
                    }
                    if let Some(output) = &call.raw_output {
                        render_json_block(
                            ui,
                            ("tool_json", call.tool_call_id.clone(), "output"),
                            "output",
                            output,
                        );
                    }
                });
        }
        SessionUpdate::ToolCallUpdate(update) => {
            let status = update.fields.status.unwrap_or(ToolCallStatus::Pending);
            let status_color = match status {
                ToolCallStatus::Pending => MOCHA.overlay2,
                ToolCallStatus::InProgress => MOCHA.yellow,
                ToolCallStatus::Completed => MOCHA.green,
                ToolCallStatus::Failed => MOCHA.red,
                _ => MOCHA.overlay2,
            };
            let title = update.fields.title.as_deref().unwrap_or("tool update");
            ui.label(
                egui::RichText::new(format!("{title} ({status:?})"))
                    .color(status_color)
                    .size(12.0),
            );
        }
        SessionUpdate::Plan(plan) => {
            super::plan::render_plan_timeline_item(ui, plan);
        }
        SessionUpdate::AvailableCommandsUpdate(_) => {
            ui.label(
                egui::RichText::new("Available commands updated")
                    .color(MOCHA.subtext0)
                    .size(12.0),
            );
        }
        SessionUpdate::CurrentModeUpdate(mode) => {
            ui.label(
                egui::RichText::new(format!("Mode: {}", mode.current_mode_id))
                    .color(MOCHA.subtext0)
                    .size(12.0),
            );
        }
        _ => {
            ui.label(
                egui::RichText::new(format!("{update:?}"))
                    .color(MOCHA.subtext0)
                    .monospace()
                    .size(11.0),
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
            let mut rt = egui::RichText::new(&text.text).color(color).size(12.0);
            if italics {
                rt = rt.italics();
            }
            ui.label(rt);
        }
        other => {
            let mut rt = egui::RichText::new(format!("{other:?}"))
                .color(color)
                .monospace()
                .size(11.0);
            if italics {
                rt = rt.italics();
            }
            ui.label(rt);
        }
    }
}

fn render_json_block<S: std::hash::Hash + Clone>(
    ui: &mut egui::Ui,
    id_salt: S,
    label: &str,
    value: &serde_json::Value,
) {
    ui.add_space(2.0); // Keep 2.0 as this is a custom spacing value
    ui.label(
        egui::RichText::new(label.to_uppercase())
            .color(MOCHA.subtext0)
            .size(11.0)
            .strong(),
    );
    let pretty = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    let mut text = pretty;
    egui::ScrollArea::vertical()
        .id_salt(("json_scroll", id_salt.clone()))
        .max_height(160.0)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .id_salt(("json_text", id_salt.clone()))
                    .font(egui::FontId::monospace(11.0))
                    .desired_rows(6)
                    .desired_width(ui.available_width())
                    .interactive(false),
            );
        });
}
