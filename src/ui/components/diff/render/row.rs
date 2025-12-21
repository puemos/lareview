use super::super::{
    DiffAction,
    model::{ChangeType, LineContext},
};
use super::types::DiffLineInfo;
use super::utils::{DIFF_FONT_SIZE, paint_inline_text_job};
use crate::ui::{spacing, theme};
use eframe::egui::{self, FontId, TextFormat, text::LayoutJob};
use egui_phosphor::regular::PLUS;

pub fn render_unified_row(
    ui: &mut egui::Ui,
    line: &DiffLineInfo,
    ctx: LineContext,
    is_active: bool,
    on_comment_click: Option<&dyn Fn(usize, usize, usize)>,
) -> DiffAction {
    let theme = theme::current_theme();
    let mut action = DiffAction::None;

    let comment_open_id = ui.id().with(("comment_open", ctx.file_idx, ctx.line_idx));
    let mut is_comment_active =
        ui.memory(|mem| mem.data.get_temp::<bool>(comment_open_id).unwrap_or(false));

    let line_number = match line.change_type {
        ChangeType::Equal | ChangeType::Delete => line.old_line_num,
        ChangeType::Insert => line.new_line_num,
    };

    let (prefix, mut bg_color, text_color, mut line_num_bg) = match line.change_type {
        ChangeType::Equal => (
            " ",
            theme.transparent,
            theme.text_primary,
            theme.transparent,
        ),
        ChangeType::Delete => (
            "-",
            theme.destructive.gamma_multiply(0.15),
            theme.destructive,
            theme.destructive.gamma_multiply(0.25),
        ),
        ChangeType::Insert => (
            "+",
            theme.success.gamma_multiply(0.15),
            theme.success,
            theme.success.gamma_multiply(0.25),
        ),
    };

    if is_active {
        let active_color = theme.accent.gamma_multiply(0.2);
        bg_color = active_color;
        line_num_bg = active_color;
    }

    let main_response = egui::Frame::NONE
        .fill(bg_color)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.set_width(ui.available_width());

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;

                let _line_numbers_frame = egui::Frame::NONE
                    .fill(line_num_bg)
                    .inner_margin(egui::Margin::symmetric(spacing::SPACING_XS as i8, 0))
                    .show(ui, |ui| {
                        let line_numbers = match line.change_type {
                            ChangeType::Equal => match (line.old_line_num, line.new_line_num) {
                                (Some(old), Some(new)) => format!("{:>4} {:>4}", old, new),
                                _ => "         ".to_string(),
                            },
                            ChangeType::Delete => {
                                if let Some(old) = line.old_line_num {
                                    format!("{:>4}     ", old)
                                } else {
                                    "         ".to_string()
                                }
                            }
                            ChangeType::Insert => {
                                if let Some(new) = line.new_line_num {
                                    format!("     {:>4}", new)
                                } else {
                                    "         ".to_string()
                                }
                            }
                        };

                        ui.label(
                            egui::RichText::new(line_numbers)
                                .font(FontId::monospace(DIFF_FONT_SIZE))
                                .color(theme.text_disabled),
                        );
                    });

                egui::Frame::NONE
                    .fill(bg_color)
                    .inner_margin(egui::Margin::symmetric(spacing::SPACING_XS as i8, 0))
                    .show(ui, |ui| {
                        let mut job = LayoutJob::default();

                        job.append(
                            prefix,
                            0.0,
                            TextFormat {
                                font_id: FontId::monospace(DIFF_FONT_SIZE),
                                color: text_color,
                                ..Default::default()
                            },
                        );
                        job.append(
                            " ",
                            0.0,
                            TextFormat {
                                font_id: FontId::monospace(DIFF_FONT_SIZE),
                                color: text_color,
                                ..Default::default()
                            },
                        );

                        let content = if line.content.len() > 1000 {
                            format!(
                                "{}...",
                                &line.content[..std::cmp::min(1000, line.content.len())]
                            )
                        } else {
                            line.content.clone()
                        };

                        if let Some(segments) = &line.inline_segments {
                            let highlight_bg = match line.change_type {
                                ChangeType::Delete => theme.destructive.gamma_multiply(0.4),
                                ChangeType::Insert => theme.success.gamma_multiply(0.4),
                                ChangeType::Equal => theme.transparent,
                            };
                            paint_inline_text_job(&mut job, segments, text_color, highlight_bg);
                        } else {
                            job.append(
                                &content,
                                0.0,
                                TextFormat {
                                    font_id: FontId::monospace(DIFF_FONT_SIZE),
                                    color: text_color,
                                    ..Default::default()
                                },
                            );
                        }

                        ui.label(job);
                    });
            });
        })
        .response;

    if main_response.hovered() && line_number.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    let line_number_for_action = line_number;
    let mut comment_button_rect: Option<egui::Rect> = None;

    let pointer_pos = ui.ctx().input(|i| i.pointer.hover_pos());
    let is_line_hovered = pointer_pos.is_some_and(|p| main_response.rect.contains(p));

    let show_comment_button = line_number.is_some() && !is_comment_active && is_line_hovered;

    if show_comment_button {
        let row_rect = main_response.rect;
        let size: f32 = 18.0;
        let button_size = egui::vec2(size, size);
        let offset_x: f32 = -4.0;
        let top_y = row_rect.center().y - size * 0.5 - 1.0;
        let button_rect =
            egui::Rect::from_min_size(egui::pos2(row_rect.left() + offset_x, top_y), button_size);
        comment_button_rect = Some(button_rect);

        let painter = ui.painter();
        painter.rect_filled(button_rect, size * 0.5, theme.accent);
        painter.text(
            button_rect.center(),
            egui::Align2::CENTER_CENTER,
            PLUS.to_string(),
            FontId::proportional(size - 2.0),
            theme.bg_primary,
        );

        let hovered_button = pointer_pos.is_some_and(|p| button_rect.contains(p));
        let clicked = ui
            .ctx()
            .input(|i| hovered_button && i.pointer.button_clicked(egui::PointerButton::Primary));

        if clicked {
            ui.memory_mut(|mem| {
                mem.data.insert_temp(comment_open_id, true);
            });
            is_comment_active = true;

            if let Some(num) = line_number_for_action {
                if let Some(callback) = on_comment_click {
                    callback(ctx.file_idx, ctx.line_idx, num);
                }
                action = DiffAction::AddNote {
                    file_idx: ctx.file_idx,
                    line_idx: ctx.line_idx,
                    line_number: num,
                    file_path: ctx.file_path.clone(),
                };
            }
        }
    }

    if let Some(pos) = pointer_pos {
        if let Some(btn_rect) = comment_button_rect {
            if btn_rect.contains(pos) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            } else if main_response.rect.contains(pos) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
            }
        } else if main_response.rect.contains(pos) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
        }
    }

    if is_comment_active {
        ui.add_space(4.0);
        let text_edit_id = ui.id().with(("comment_text", ctx.file_idx, ctx.line_idx));
        let mut comment_text = ui.memory(|mem| {
            mem.data
                .get_temp::<String>(text_edit_id)
                .unwrap_or_default()
        });

        let text_response = ui.add(
            egui::TextEdit::multiline(&mut comment_text)
                .id_salt(text_edit_id)
                .hint_text("Enter your comment...")
                .font(egui::TextStyle::Monospace)
                .frame(false)
                .desired_rows(3)
                .desired_width(ui.available_width())
                .lock_focus(true),
        );

        ui.memory_mut(|mem| mem.data.insert_temp(text_edit_id, comment_text.clone()));

        ui.horizontal(|ui| {
            if ui.button("Save Comment").clicked() {
                if let Some(line_num) = line_number {
                    action = DiffAction::SaveNote {
                        file_idx: ctx.file_idx,
                        line_idx: ctx.line_idx,
                        line_number: line_num,
                        note_text: comment_text.clone(),
                    };
                }

                ui.memory_mut(|mem| {
                    mem.data.remove::<String>(text_edit_id);
                    mem.data.insert_temp(comment_open_id, false);
                });
                is_comment_active = false;
            }

            if ui.button("Cancel").clicked() {
                ui.memory_mut(|mem| {
                    mem.data.remove::<String>(text_edit_id);
                    mem.data.insert_temp(comment_open_id, false);
                });
                is_comment_active = false;
            }
        });

        if text_response.gained_focus() {
            ui.ctx().memory_mut(|mem| {
                mem.request_focus(text_response.id);
            });
        }
    }

    action
}
