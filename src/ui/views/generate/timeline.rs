use crate::ui::app::{TimelineContent, TimelineItem};
use crate::ui::spacing;
use catppuccin_egui::MOCHA;
use eframe::egui;
use eframe::egui::collapsing_header::CollapsingState;

use agent_client_protocol::{ContentBlock, SessionUpdate, ToolCallStatus};

fn truncate_end(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_owned();
    }
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i + 1 >= max_chars {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

fn strip_json_suffix(s: &str) -> &str {
    if let Some(i) = s.find(": {") {
        s[..i].trim()
    } else {
        s.trim()
    }
}

pub(super) fn render_timeline_item(ui: &mut egui::Ui, item: &TimelineItem) {
    // FIX: Set a maximum width for this item to force wrapping logic to kick in.
    ui.set_max_width(ui.available_width());

    match &item.content {
        TimelineContent::LocalLog(line) => {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(line)
                        .color(MOCHA.subtext0)
                        .monospace()
                        .size(12.0),
                )
                .wrap(), // Fixed: .wrap() takes no arguments now
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
            ToolCallStatus::Pending => (MOCHA.overlay2, "pending"),
            ToolCallStatus::InProgress => (MOCHA.yellow, "in progress"),
            ToolCallStatus::Completed => (MOCHA.green, "completed"),
            ToolCallStatus::Failed => (MOCHA.red, "failed"),
            _ => (MOCHA.overlay2, "unknown"),
        }
    };

    match update {
        SessionUpdate::UserMessageChunk(chunk) | SessionUpdate::AgentMessageChunk(chunk) => {
            render_content_chunk(ui, chunk, MOCHA.text, false);
        }
        SessionUpdate::AgentThoughtChunk(chunk) => {
            ui.indent("thought_indent", |ui| {
                render_content_chunk(ui, chunk, MOCHA.sky, true);
            });
        }
        SessionUpdate::ToolCall(call) => {
            ui.group(|ui| {
                ui.set_max_width(ui.available_width());

                let (status_color, status_label) = get_status_style(&call.status);

                // Build full string for hover, but show a short one
                let full = format!("{} ({})", call.title, status_label);
                let cleaned = strip_json_suffix(&full).to_owned();
                let shown = truncate_end(&cleaned, 80);

                let header_rt = egui::RichText::new(shown)
                    .color(status_color)
                    .strong()
                    .size(12.0);

                let id = ui.make_persistent_id(("tool_call", &call.tool_call_id));

                // collapsed by default
                let default_open = false;

                CollapsingState::load_with_default_open(ui.ctx(), id, default_open)
                    .show_header(ui, |ui| {
                        // Force left aligned layout so it does not center in wide headers
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.add(egui::Label::new(header_rt).wrap());
                        });
                    })
                    .body(|ui| {
                        // your body stays the same
                        ui.spacing_mut().item_spacing = egui::vec2(
                            spacing::TIGHT_ITEM_SPACING.0,
                            spacing::TIGHT_ITEM_SPACING.1,
                        );

                        if let Some(kind) =
                            (!matches!(call.kind, agent_client_protocol::ToolKind::Other))
                                .then_some(call.kind)
                        {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("kind:")
                                        .size(11.0)
                                        .color(MOCHA.overlay1),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{kind:?}"))
                                        .size(11.0)
                                        .color(MOCHA.subtext0),
                                );
                            });
                        }

                        if let Some(input) = &call.raw_input {
                            render_json_block(
                                ui,
                                ("tool_json", call.tool_call_id.clone(), "input"),
                                "Input",
                                input,
                            );
                        }
                        if let Some(output) = &call.raw_output {
                            render_json_block(
                                ui,
                                ("tool_json", call.tool_call_id.clone(), "output"),
                                "Output",
                                output,
                            );
                        }
                    });
            });
        }
        SessionUpdate::ToolCallUpdate(update) => {
            let status = update.fields.status.unwrap_or(ToolCallStatus::Pending);
            let (status_color, _) = get_status_style(&status);
            let title = update.fields.title.as_deref().unwrap_or("tool update");

            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new("Update:")
                        .color(MOCHA.subtext0)
                        .size(12.0),
                );

                ui.add(
                    egui::Label::new(
                        egui::RichText::new(format!("{title} -> {status:?}"))
                            .color(status_color)
                            .size(12.0),
                    )
                    .wrap(), // Fixed
                );
            });
        }
        SessionUpdate::Plan(plan) => {
            super::plan::render_plan_timeline_item(ui, plan);
        }
        SessionUpdate::AvailableCommandsUpdate(_) => {
            ui.add(
                egui::Label::new(
                    egui::RichText::new("Available commands updated")
                        .color(MOCHA.overlay1)
                        .italics()
                        .size(12.0),
                )
                .wrap(), // Fixed
            );
        }
        SessionUpdate::CurrentModeUpdate(mode) => {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Mode switch:")
                        .color(MOCHA.overlay1)
                        .size(12.0),
                );
                ui.label(
                    // Fixed: Added .to_string()
                    egui::RichText::new(mode.current_mode_id.to_string())
                        .color(MOCHA.text)
                        .strong()
                        .size(12.0),
                );
            });
        }
        _ => {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(format!("{update:?}"))
                        .color(MOCHA.subtext0)
                        .monospace()
                        .size(11.0),
                )
                .wrap(), // Fixed
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
            ui.add(egui::Label::new(rt).wrap()); // Fixed
        }
        other => {
            let mut rt = egui::RichText::new(format!("{other:?}"))
                .color(color)
                .monospace()
                .size(11.0);
            if italics {
                rt = rt.italics();
            }
            ui.add(egui::Label::new(rt).wrap()); // Fixed
        }
    }
}

fn render_json_block<S: std::hash::Hash + Clone>(
    ui: &mut egui::Ui,
    id_salt: S,
    label: &str,
    value: &serde_json::Value,
) {
    ui.add_space(spacing::TIGHT_ITEM_SPACING.1);
    ui.label(
        egui::RichText::new(label)
            .color(MOCHA.overlay1)
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
                    .code_editor()
                    .desired_rows(6)
                    .desired_width(ui.available_width())
                    .interactive(false)
                    .frame(true),
            );
        });
}
