use super::super::{
    DiffAction,
    model::{ChangeType, LineContext},
};
use super::types::DiffLineInfo;
use super::utils::{DIFF_FONT_SIZE, paint_inline_text_job};
use crate::ui::{spacing, theme};
use eframe::egui::{self, FontId, TextFormat, text::LayoutJob};
use egui_phosphor::regular::{ARROW_SQUARE_OUT, PENCIL};

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
            theme.destructive.gamma_multiply(0.05),
            theme.destructive,
            theme.destructive.gamma_multiply(0.05),
        ),
        ChangeType::Insert => (
            "+",
            theme.success.gamma_multiply(0.05),
            theme.success,
            theme.success.gamma_multiply(0.05),
        ),
    };

    if is_active {
        let active_color = theme.accent.gamma_multiply(0.1);
        bg_color = active_color;
        line_num_bg = active_color;
    }

    struct TextOverlay {
        pos: egui::Pos2,
        width: f32,
        job: LayoutJob,
    }

    let mut text_overlay: Option<TextOverlay> = None;
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

                        let label =
                            egui::Label::new(job.clone()).wrap_mode(egui::TextWrapMode::Wrap);
                        let response = ui.add(label);
                        text_overlay = Some(TextOverlay {
                            pos: response.rect.min,
                            width: response.rect.width(),
                            job,
                        });
                    });
            });
        })
        .response;

    let line_number_for_action = line_number;
    let mut comment_button_rect: Option<egui::Rect> = None;
    let mut open_button_rect: Option<egui::Rect> = None;

    let pointer_pos = ui.ctx().input(|i| i.pointer.hover_pos());
    let is_line_hovered = pointer_pos.is_some_and(|p| main_response.rect.contains(p));
    let is_modifier_pressed = ui.ctx().input(|i| i.modifiers.ctrl || i.modifiers.command);

    let show_comment_button = line_number.is_some() && !is_comment_active && is_line_hovered;
    let show_open_button = line_number.is_some() && is_line_hovered;

    if show_comment_button {
        let row_rect = main_response.rect;
        let size: f32 = 18.0;
        let gap: f32 = 4.0;
        let right_offset: f32 = 4.0;
        let button_size = egui::vec2(size, size);
        let top_y = row_rect.center().y - size * 0.5 - 1.0;
        let open_x = row_rect.right() - size - right_offset;
        let comment_x = open_x - gap - size;
        let button_rect = egui::Rect::from_min_size(egui::pos2(comment_x, top_y), button_size);
        comment_button_rect = Some(button_rect);

        let painter = ui.painter();
        painter.rect_filled(button_rect, size * 0.5, theme.accent);
        painter.text(
            button_rect.center(),
            egui::Align2::CENTER_CENTER,
            PENCIL.to_string(),
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

    if show_open_button {
        let row_rect = main_response.rect;
        let size: f32 = 18.0;
        let right_offset: f32 = 4.0;
        let button_size = egui::vec2(size, size);
        let top_y = row_rect.center().y - size * 0.5 - 1.0;
        let button_rect = egui::Rect::from_min_size(
            egui::pos2(row_rect.right() - size - right_offset, top_y),
            button_size,
        );
        open_button_rect = Some(button_rect);

        let painter = ui.painter();
        painter.rect_filled(button_rect, size * 0.5, theme.brand);
        painter.text(
            button_rect.center(),
            egui::Align2::CENTER_CENTER,
            ARROW_SQUARE_OUT.to_string(),
            FontId::proportional(size - 4.0),
            theme.bg_primary,
        );

        let hovered_button = pointer_pos.is_some_and(|p| button_rect.contains(p));
        let clicked = ui
            .ctx()
            .input(|i| hovered_button && i.pointer.button_clicked(egui::PointerButton::Primary));

        if clicked
            && matches!(action, DiffAction::None)
            && let Some(num) = line_number_for_action
        {
            action = DiffAction::OpenInEditor {
                file_path: ctx.file_path.clone(),
                line_number: num,
            };
        }
    }

    let show_jump_hint = is_line_hovered && is_modifier_pressed && line_number_for_action.is_some();
    if show_jump_hint {
        let tint_color = match line.change_type {
            ChangeType::Equal => theme.brand,
            ChangeType::Delete | ChangeType::Insert => theme.text_primary,
        };
        if let Some(overlay) = &text_overlay {
            let mut job = overlay.job.clone();
            job.wrap.max_width = overlay.width;
            for (idx, section) in job.sections.iter_mut().enumerate() {
                if idx >= 2 {
                    section.format.color = tint_color;
                }
            }
            let galley = ui.painter().layout_job(job);
            ui.painter().add(egui::Shape::Text(egui::epaint::TextShape {
                pos: overlay.pos,
                galley,
                underline: egui::Stroke::NONE,
                override_text_color: Some(tint_color),
                angle: 0.0,
                fallback_color: tint_color,
                opacity_factor: 1.0,
            }));
        }
    }

    if let Some(pos) = pointer_pos {
        let hover_comment = comment_button_rect.is_some_and(|rect| rect.contains(pos));
        let hover_open = open_button_rect.is_some_and(|rect| rect.contains(pos));
        let hover_row = main_response.rect.contains(pos);

        if hover_comment || hover_open || (hover_row && is_modifier_pressed) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        } else if hover_row {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
        }
    }

    let is_primary_clicked = ui
        .ctx()
        .input(|i| i.pointer.button_clicked(egui::PointerButton::Primary));
    let clicked_in_row = pointer_pos.is_some_and(|p| main_response.rect.contains(p));
    let clicked_comment_button = comment_button_rect
        .zip(pointer_pos)
        .is_some_and(|(rect, pos)| rect.contains(pos));
    let clicked_open_button = open_button_rect
        .zip(pointer_pos)
        .is_some_and(|(rect, pos)| rect.contains(pos));

    if matches!(action, DiffAction::None)
        && !is_comment_active
        && is_modifier_pressed
        && is_primary_clicked
        && clicked_in_row
        && !clicked_comment_button
        && !clicked_open_button
        && line_number_for_action.is_some()
        && let Some(num) = line_number_for_action
    {
        action = DiffAction::OpenInEditor {
            file_path: ctx.file_path.clone(),
            line_number: num,
        };
    }

    action
}
