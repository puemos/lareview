use crate::ui::app::{TimelineContent, TimelineItem};
use crate::ui::theme::current_theme;
use crate::ui::{icons, typography};
use eframe::egui;
use eframe::egui::collapsing_header::CollapsingState;

use agent_client_protocol::{ContentBlock, SessionUpdate, ToolCallStatus};

pub(super) fn render_timeline_item(ui: &mut egui::Ui, item: &TimelineItem) {
    ui.set_max_width(ui.available_width());

    match &item.content {
        TimelineContent::LocalLog(_line) => {
            // Internal technical logs are not rendered in the user-facing activity log
        }
        TimelineContent::Update(update) => {
            render_session_update(ui, update);
        }
    }
}

fn render_session_update(ui: &mut egui::Ui, update: &SessionUpdate) {
    let get_status_style = |status: &ToolCallStatus| -> (egui::Color32, &str) {
        match status {
            ToolCallStatus::Pending => (current_theme().text_muted, "Pending"),
            ToolCallStatus::InProgress => (current_theme().warning, "Running"),
            ToolCallStatus::Completed => (current_theme().success, "Done"),
            ToolCallStatus::Failed => (current_theme().destructive, "Failed"),
            _ => (current_theme().text_muted, "Unknown"),
        }
    };

    match update {
        SessionUpdate::UserMessageChunk(chunk) => {
            ui.horizontal(|ui| {
                ui.label(
                    typography::bold("User")
                        .color(current_theme().text_accent)
                        .size(12.0),
                );
                render_content_chunk(ui, chunk, current_theme().text_primary, false);
            });
        }
        SessionUpdate::AgentMessageChunk(chunk) => {
            render_content_chunk(ui, chunk, current_theme().text_primary, false);
        }
        SessionUpdate::AgentThoughtChunk(chunk) => {
            // New Design: No quotes, distinct color
            render_content_chunk(ui, chunk, current_theme().accent, false);
        }
        SessionUpdate::ToolCall(call) => {
            ui.push_id((&call.tool_call_id, "tool_card"), |ui| {
                egui::Frame::group(ui.style())
                    .fill(current_theme().bg_secondary)
                    .stroke(egui::Stroke::new(1.0, current_theme().border))
                    .corner_radius(crate::ui::spacing::RADIUS_MD)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.set_max_width(ui.available_width());

                        let (status_color, status_label) = get_status_style(&call.status);
                        let tool_label =
                            tool_label_from_parts(Some(&call.title), call.raw_input.as_ref());

                        let id = ui.make_persistent_id("collapsing");
                        let default_open = false;

                        CollapsingState::load_with_default_open(ui.ctx(), id, default_open)
                            .show_header(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    let full_label =
                                        format!("{}: {}", tool_label.server, tool_label.tool);
                                    let truncated_label = if full_label.len() > 40 {
                                        format!("{}...", &full_label[..37])
                                    } else {
                                        full_label
                                    };

                                    ui.label(
                                        typography::bold_label(truncated_label)
                                            .family(egui::FontFamily::Monospace)
                                            .color(current_theme().text_primary),
                                    );

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                typography::small_mono(format!("[{}]", status_label))
                                                    .color(status_color)
                                                    .size(11.0),
                                            );

                                            if matches!(call.status, ToolCallStatus::InProgress) {
                                                ui.add_space(4.0);
                                                crate::ui::animations::cyber::cyber_spinner(
                                                    ui,
                                                    current_theme().brand,
                                                    Some(
                                                        crate::ui::animations::cyber::CyberSpinnerSize::Sm,
                                                    ),
                                                );
                                            }
                                        },
                                    );
                                });
                            })
                            .body(|ui| {
                                ui.add_space(4.0);

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
            let tool_label = tool_label_from_parts(
                update.fields.title.as_deref(),
                update.fields.raw_input.as_ref(),
            );

            ui.horizontal(|ui| {
                let full_label = format!("{}: {}", tool_label.server, tool_label.tool);
                let truncated_label = if full_label.len() > 40 {
                    format!("{}...", &full_label[..37])
                } else {
                    full_label
                };

                ui.label(
                    typography::small_mono(truncated_label)
                        .color(current_theme().text_muted)
                        .size(12.0),
                );

                if matches!(status, ToolCallStatus::InProgress) {
                    ui.add_space(4.0);
                    crate::ui::animations::cyber::cyber_spinner(
                        ui,
                        current_theme().brand,
                        Some(crate::ui::animations::cyber::CyberSpinnerSize::Sm),
                    );
                }

                ui.label(
                    typography::small_mono(format!("[{}]", label))
                        .color(color)
                        .size(12.0),
                );
            });
        }
        SessionUpdate::Plan(protocol_plan) => {
            let plan = crate::domain::Plan::from(protocol_plan.clone());
            // Collapsed by default
            let id = ui.make_persistent_id(format!("plan_{}", plan.entries.len()));
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
                .show_header(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            typography::body(icons::ICON_PLAN).color(current_theme().text_accent),
                        );
                        ui.label(
                            typography::bold_label(format!(
                                "Review Plan ({} steps)",
                                plan.entries.len()
                            ))
                            .family(egui::FontFamily::Monospace)
                            .color(current_theme().text_primary),
                        );
                    });
                })
                .body(|ui| {
                    super::plan::render_plan_timeline_item(ui, &plan);
                });
        }
        SessionUpdate::AvailableCommandsUpdate(_) => {
            // Skip internal system log: "System: Commands updated"
        }
        SessionUpdate::CurrentModeUpdate(_) => {
            // Skip internal mode switch logs
        }
        _ => {
            ui.add(
                egui::Label::new(
                    typography::small_mono(format!("{:?}", update))
                        .color(current_theme().text_muted),
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
            let mut rt = typography::mono(&text.text).color(color).size(13.0); // Slightly larger
            if italics {
                rt = rt.italics();
            }
            ui.add(egui::Label::new(rt).wrap());
        }
        other => {
            let mut rt = typography::small_mono(format!("{:?}", other))
                .color(color)
                .size(11.0);
            if italics {
                rt = rt.italics();
            }
            ui.add(egui::Label::new(rt).wrap());
        }
    }
}

struct ToolLabel {
    server: String,
    tool: String,
}

fn tool_label_from_parts(title: Option<&str>, raw_input: Option<&serde_json::Value>) -> ToolLabel {
    if let Some(input) = raw_input.and_then(parse_tool_label_from_payload) {
        return input;
    }

    if let Some(title) = title {
        if let Some(label) = parse_tool_label_from_title(title) {
            return label;
        }

        let trimmed = title.trim();
        if !trimmed.is_empty() {
            let server = fallback_server_for_tool(trimmed).unwrap_or("local");
            return ToolLabel {
                server: server.to_string(),
                tool: trimmed.to_string(),
            };
        }
    }

    ToolLabel {
        server: "local".to_string(),
        tool: "tool".to_string(),
    }
}

fn parse_tool_label_from_payload(payload: &serde_json::Value) -> Option<ToolLabel> {
    let parsed = if let Some(raw) = payload.as_str() {
        serde_json::from_str::<serde_json::Value>(raw).ok()?
    } else {
        payload.clone()
    };

    let tool = parsed
        .get("tool")
        .or_else(|| parsed.get("name"))
        .and_then(|value| value.as_str());
    let server = parsed.get("server").and_then(|value| value.as_str());

    if let Some(tool) = tool {
        let server = server.or_else(|| fallback_server_for_tool(tool))?;
        return Some(ToolLabel {
            server: server.to_string(),
            tool: tool.to_string(),
        });
    }

    None
}

fn parse_tool_label_from_title(title: &str) -> Option<ToolLabel> {
    let trimmed = title.trim();
    if trimmed.is_empty() || trimmed.starts_with('{') || trimmed.starts_with('[') {
        return None;
    }

    // Example: "return_task (lareview-tasks MCP Server): {...}"
    if let Some((tool_part, rest)) = trimmed.split_once('(')
        && let Some((server_part, _after_paren)) = rest.split_once(')')
    {
        let tool = tool_part.trim();

        // Normalize server: strip common suffixes like "MCP Server"
        let server = server_part
            .trim()
            .strip_suffix("MCP Server")
            .unwrap_or(server_part.trim())
            .trim();

        if !tool.is_empty() && !server.is_empty() {
            return Some(ToolLabel {
                server: server.to_string(),
                tool: tool.to_string(),
            });
        }
    }

    if let Some((server, tool)) = trimmed.split_once('/') {
        return Some(ToolLabel {
            server: server.trim().to_string(),
            tool: tool.trim().to_string(),
        });
    }

    if let Some((server, tool)) = trimmed.split_once(':') {
        let server = server.trim();
        let tool = tool.trim();
        if !server.is_empty() && !tool.is_empty() {
            return Some(ToolLabel {
                server: server.to_string(),
                tool: tool.to_string(),
            });
        }
    }

    let server = fallback_server_for_tool(trimmed)?;
    Some(ToolLabel {
        server: server.to_string(),
        tool: trimmed.to_string(),
    })
}

fn fallback_server_for_tool(tool: &str) -> Option<&'static str> {
    match tool {
        "return_task" | "return_plans" | "finalize_review" | "repo_search" | "repo_list_files" => {
            Some("lareview-tasks")
        }
        _ => None,
    }
}

fn render_kv_json(ui: &mut egui::Ui, label: &str, value: &serde_json::Value) {
    ui.label(
        typography::bold_label(label)
            .color(current_theme().text_muted)
            .size(11.0),
    );

    if let serde_json::Value::Object(map) = value {
        egui::Grid::new(ui.id().with(label))
            .num_columns(2)
            .striped(true)
            .min_col_width(60.0)
            .max_col_width(ui.available_width() - 80.0) // Leave room for key column
            .show(ui, |ui| {
                for (k, v) in map {
                    ui.label(typography::small_mono(k).color(current_theme().accent));

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
                            typography::small_mono(shown_v).color(current_theme().text_muted),
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
                .font(typography::mono_font(11.0))
                .code_editor()
                .desired_rows(4)
                .desired_width(ui.available_width())
                .interactive(false),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    #[test]
    fn test_render_timeline_local_log() {
        let item = TimelineItem {
            seq: 1,
            stream_key: None,
            content: TimelineContent::LocalLog("System starting".to_string()),
        };
        let mut harness = Harness::new_ui(|ui| {
            render_timeline_item(ui, &item);
        });
        harness.run();
        // Technical logs should NOT be rendered
        let dump = format!("{:?}", harness);
        assert!(
            !dump.contains("System starting"),
            "Technical log should not be rendered"
        );
    }

    #[test]
    fn test_render_timeline_agent_chunk() {
        let chunk_json = serde_json::json!({
            "content": { "type": "text", "text": "Thinking..." }
        });
        let chunk: agent_client_protocol::ContentChunk =
            serde_json::from_value(chunk_json).unwrap();
        let item = TimelineItem {
            seq: 1,
            stream_key: None,
            content: TimelineContent::Update(Box::new(SessionUpdate::AgentMessageChunk(chunk))),
        };
        let mut harness = Harness::new(|ctx| {
            crate::ui::app::LaReviewApp::setup_fonts(ctx);
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.style_mut().override_font_id = Some(egui::FontId::proportional(12.0));
                render_timeline_item(ui, &item);
            });
        });
        harness.run_steps(5);
        harness.get_by_label("Thinking...");
    }

    #[test]
    fn test_render_timeline_tool_call() {
        let call_json = serde_json::json!({
            "toolCallId": "tc1",
            "title": "search",
            "kind": "other",
            "status": "in_progress",
            "content": [],
            "locations": []
        });
        let call: agent_client_protocol::ToolCall = serde_json::from_value(call_json).unwrap();
        let item = TimelineItem {
            seq: 1,
            stream_key: None,
            content: TimelineContent::Update(Box::new(SessionUpdate::ToolCall(call))),
        };
        let mut harness = Harness::new(|ctx| {
            crate::ui::app::LaReviewApp::setup_fonts(ctx);
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.style_mut().override_font_id = Some(egui::FontId::proportional(12.0));
                render_timeline_item(ui, &item);
            });
        });
        harness.run_steps(5);
        harness
            .get_all_by_role(egui::accesskit::Role::Label)
            .into_iter()
            .find(|n| format!("{:?}", n).contains("search"))
            .expect("Tool title not found");
        harness
            .get_all_by_role(egui::accesskit::Role::Label)
            .into_iter()
            .find(|n| format!("{:?}", n).contains("Running"))
            .expect("Status not found");
    }

    #[test]
    fn test_tool_label_parsing() {
        // Case 1: Simple tool
        let label1 = tool_label_from_parts(Some("return_task"), None);
        assert_eq!(label1.server, "lareview-tasks");
        assert_eq!(label1.tool, "return_task");

        // Case 2: Tool with server in title
        let label2 = tool_label_from_parts(Some("my-server/my-tool"), None);
        assert_eq!(label2.server, "my-server");
        assert_eq!(label2.tool, "my-tool");

        // Case 3: Complex title
        let label3 = tool_label_from_parts(
            Some("return_task (lareview-tasks MCP Server): { ... }"),
            None,
        );
        assert_eq!(label3.server, "lareview-tasks");
        assert_eq!(label3.tool, "return_task");
    }

    #[test]
    fn test_render_timeline_plan() {
        let plan_json = serde_json::json!({
            "entries": [
                { "content": "Step 1", "status": "pending", "priority": "medium" }
            ]
        });
        let plan: agent_client_protocol::Plan = serde_json::from_value(plan_json).unwrap();
        let item = TimelineItem {
            seq: 1,
            stream_key: None,
            content: TimelineContent::Update(Box::new(SessionUpdate::Plan(plan))),
        };
        let mut harness = Harness::new_ui(|ui| {
            render_timeline_item(ui, &item);
        });
        harness.run();
        harness.get_by_label("Review Plan (1 steps)");
    }
}
