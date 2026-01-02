use super::super::{
    DiffAction,
    model::{ChangeType, LineContext},
};
use super::types::DiffLineInfo;
use super::utils::{DIFF_FONT_SIZE, paint_inline_text_job};
use crate::domain::FeedbackSide;
use crate::ui::{icons, spacing, theme};
use eframe::egui::{self, FontId, TextFormat, text::LayoutJob};

pub fn render_unified_row(
    ui: &mut egui::Ui,
    line: &DiffLineInfo,
    ctx: LineContext,
    is_active: bool,
    show_actions: bool,
    on_comment_click: Option<&dyn Fn(usize, usize, usize)>,
) -> DiffAction {
    let theme = theme::current_theme();
    let mut action = DiffAction::None;

    let line_number = match line.change_type {
        ChangeType::Equal | ChangeType::Delete => line.old_line_num,
        ChangeType::Insert => line.new_line_num,
    };
    let side_for_action = match line.change_type {
        ChangeType::Delete => FeedbackSide::Old,
        ChangeType::Insert => FeedbackSide::New,
        ChangeType::Equal => FeedbackSide::New,
    };

    let (prefix, mut bg_color, text_color, mut line_num_bg) = match line.change_type {
        ChangeType::Equal => (
            " ",
            theme.transparent,
            theme.text_primary,
            theme.transparent,
        ),
        ChangeType::Delete => (
            "- ",
            theme.destructive.gamma_multiply(0.05),
            theme.destructive,
            theme.destructive.gamma_multiply(0.05),
        ),
        ChangeType::Insert => (
            "+ ",
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

                        if let Some(tokens) = &line.syntax_tokens {
                            for token in tokens {
                                job.append(
                                    &token.text,
                                    0.0,
                                    TextFormat {
                                        font_id: FontId::monospace(DIFF_FONT_SIZE),
                                        color: token.color,
                                        ..Default::default()
                                    },
                                );
                            }
                        } else if let Some(segments) = &line.inline_segments {
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
                        let _response = ui.add(label);
                    });
            });
        })
        .response;

    let line_number_for_action = line_number;
    let mut menu_button_rect: Option<egui::Rect> = None;

    let pointer_pos = ui.ctx().input(|i| i.pointer.hover_pos());
    let is_line_hovered = pointer_pos.is_some_and(|p| main_response.rect.contains(p));
    let is_modifier_pressed = ui.ctx().input(|i| i.modifiers.ctrl || i.modifiers.command);

    let popup_id = ui.id().with(("menu_popup", ctx.file_idx, ctx.line_idx));
    let is_popup_open = egui::Popup::is_id_open(ui.ctx(), popup_id);

    if show_actions && line_number.is_some() && (is_line_hovered || is_popup_open) {
        let size: f32 = 20.0;
        let left_offset: f32 = 74.0;
        let button_size = egui::vec2(size, size);

        // Anchor to the far left of the row
        let align_left_x = main_response.rect.min.x;
        let top_y = main_response.rect.center().y - size * 0.5;

        let button_rect =
            egui::Rect::from_min_size(egui::pos2(align_left_x + left_offset, top_y), button_size);
        menu_button_rect = Some(button_rect);

        let response = ui
            .interact(
                button_rect,
                ui.id().with(("menu", ctx.file_idx, ctx.line_idx)),
                egui::Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand);

        if response.clicked() {
            egui::Popup::toggle_id(ui.ctx(), popup_id);
        }

        let painter = ui.painter();

        // Permanent muted background as requested
        let bg_color = if response.hovered() || is_popup_open {
            theme.bg_tertiary
        } else {
            theme.bg_primary
        };
        painter.circle_filled(button_rect.center(), size * 0.5, bg_color);

        let icon_color = if response.hovered() || is_popup_open {
            theme.brand
        } else {
            theme.text_primary
        };

        painter.text(
            button_rect.center(),
            egui::Align2::CENTER_CENTER,
            egui_phosphor::regular::DOTS_THREE_CIRCLE.to_string(),
            FontId::proportional(size - 4.0),
            icon_color,
        );

        egui::Popup::new(popup_id, ui.ctx().clone(), button_rect, ui.layer_id())
            .open_memory(None)
            .show(|ui| {
                let frame_response = egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(4, 2))
                    .show(ui, |ui| {
                        ui.set_max_width(150.0);

                        if ui
                            .selectable_label(
                                false,
                                format!("{} Add Feedback", icons::TAB_FEEDBACK),
                            )
                            .clicked()
                        {
                            if let Some(num) = line_number_for_action {
                                if let Some(callback) = on_comment_click {
                                    callback(ctx.file_idx, ctx.line_idx, num);
                                }
                                action = DiffAction::AddNote {
                                    file_idx: ctx.file_idx,
                                    line_idx: ctx.line_idx,
                                    line_number: num,
                                    file_path: ctx.file_path.clone(),
                                    side: side_for_action,
                                };
                            }
                            ui.close();
                        }

                        if ui
                            .selectable_label(
                                false,
                                format!(
                                    "{} Open in Editor",
                                    egui_phosphor::regular::ARROW_SQUARE_OUT
                                ),
                            )
                            .clicked()
                        {
                            if let Some(num) = line_number_for_action {
                                action = DiffAction::OpenInEditor {
                                    file_path: ctx.file_path.clone(),
                                    line_number: num,
                                };
                            }
                            ui.close();
                        }
                    });

                // Block cursor fallthrough by making the entire popup area interactive
                ui.interact(
                    frame_response.response.rect,
                    ui.id().with("popup_bg"),
                    egui::Sense::hover(),
                )
                .on_hover_cursor(egui::CursorIcon::Default);
            });
    }

    if let Some(pos) = pointer_pos {
        let hover_menu = menu_button_rect.is_some_and(|rect| rect.contains(pos));
        let hover_row = main_response.rect.contains(pos);

        if hover_menu || (hover_row && is_modifier_pressed) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        } else if hover_row {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
        }
    }

    let is_primary_clicked = ui
        .ctx()
        .input(|i| i.pointer.button_clicked(egui::PointerButton::Primary));
    let clicked_in_row = pointer_pos.is_some_and(|p| main_response.rect.contains(p));
    let clicked_menu_button = menu_button_rect
        .zip(pointer_pos)
        .is_some_and(|(rect, pos)| rect.contains(pos));

    if matches!(action, DiffAction::None)
        && is_modifier_pressed
        && is_primary_clicked
        && clicked_in_row
        && !clicked_menu_button
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
