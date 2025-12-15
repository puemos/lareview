use super::model::{ChangeType, DiffLine, DiffState, FileDiff, Row};
use super::{DiffAction, LineContext};
use crate::ui::components::diff::parse::parse_diff_by_files;
use crate::ui::spacing;
use crate::ui::theme;
use eframe::egui::{self, FontId, TextFormat, text::LayoutJob};
use egui_phosphor::regular::PLUS;

const DIFF_FONT_SIZE: f32 = 12.0;
const HEADER_FONT_SIZE: f32 = 14.0;

fn build_row_list(files: &[FileDiff], collapsed: &[bool], ui: &egui::Ui) -> (Vec<Row>, f32) {
    let mut rows = Vec::new();
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 2.0;

    for (file_idx, file) in files.iter().enumerate() {
        rows.push(Row::FileHeader { file_idx });

        if !collapsed[file_idx] {
            for line_idx in 0..file.lines.len() {
                rows.push(Row::DiffLine { file_idx, line_idx });
            }
        }
    }

    (rows, row_height)
}

fn paint_inline_text_job(
    job: &mut LayoutJob,
    segments: &[(String, bool)],
    base_color: egui::Color32,
    highlight_bg: egui::Color32,
) {
    for (text, highlight) in segments {
        let fmt = TextFormat {
            font_id: FontId::monospace(DIFF_FONT_SIZE),
            color: base_color,
            background: if *highlight {
                highlight_bg
            } else {
                egui::Color32::TRANSPARENT
            },
            ..Default::default()
        };
        job.append(text, 0.0, fmt);
    }
}

pub fn render_diff_editor(ui: &mut egui::Ui, diff_text: &str, language: &str) -> DiffAction {
    render_diff_editor_with_options(ui, diff_text, language, true, None, None)
}

pub fn render_diff_editor_full_view(
    ui: &mut egui::Ui,
    diff_text: &str,
    language: &str,
) -> DiffAction {
    render_diff_editor_with_options(ui, diff_text, language, false, None, None)
}

pub fn render_diff_editor_with_comment_callback(
    ui: &mut egui::Ui,
    diff_text: &str,
    language: &str,
    show_full_window_button: bool,
    active_line: Option<LineContext>,
    on_comment_requested: Option<&dyn Fn(usize, usize, usize)>,
) -> DiffAction {
    render_diff_editor_with_options(
        ui,
        diff_text,
        language,
        show_full_window_button,
        active_line,
        on_comment_requested,
    )
}

pub fn render_diff_editor_with_options(
    ui: &mut egui::Ui,
    diff_text: &str,
    _language: &str,
    show_full_window_button: bool,
    active_line: Option<LineContext>,
    on_comment_requested: Option<&dyn Fn(usize, usize, usize)>,
) -> DiffAction {
    let mut action = DiffAction::None;
    let state_id = ui.id().with("diff_state");

    let mut state = ui
        .ctx()
        .memory_mut(|mem| mem.data.get_persisted::<DiffState>(state_id))
        .unwrap_or_default();

    let new_hash = egui::util::hash(diff_text.as_bytes());

    if state.last_hash != new_hash {
        match parse_diff_by_files(diff_text) {
            Ok(files) => {
                let file_count = files.len();
                state.files = files;
                state.parse_error = None;
                state.collapsed = vec![true; file_count];

                let (rows, row_height) = build_row_list(&state.files, &state.collapsed, ui);
                state.rows = rows;
                state.row_height = row_height;
            }
            Err(err) => {
                state.files.clear();
                state.rows.clear();
                state.parse_error = Some(format!("Failed to parse diff: {err}"));
            }
        }
        state.last_hash = new_hash;
        ui.ctx()
            .memory_mut(|mem| mem.data.insert_persisted(state_id, state.clone()));
    }

    if let Some(err) = &state.parse_error {
        let msg = format!("{} {}", egui_phosphor::regular::WARNING, err);
        let theme = theme::current_theme();
        ui.colored_label(theme.destructive, msg);
        return DiffAction::None;
    }

    let mut open_full = false;

    let theme = theme::current_theme();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Diff").color(theme.text_primary));

        ui.label(egui::RichText::new(format!(
            "{} {} files",
            egui_phosphor::regular::FILES,
            state.files.len()
        )));

        if show_full_window_button {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(
                        egui::RichText::new(format!(
                            "{} Open",
                            egui_phosphor::regular::ARROW_SQUARE_OUT
                        ))
                        .color(theme.brand),
                    )
                    .clicked()
                {
                    open_full = true;
                }
            });
        }
    });

    ui.add_space(4.0);

    let total_rows = state.rows.len();
    let row_height = state.row_height;

    egui::ScrollArea::vertical()
        .id_salt("diff_scroll")
        .auto_shrink([false; 2])
        .show_rows(ui, row_height, total_rows, |ui, range| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

            for idx in range {
                if idx >= state.rows.len() {
                    break;
                }

                match state.rows[idx].clone() {
                    Row::FileHeader { file_idx } => {
                        render_file_header(ui, file_idx, &mut state, state_id);
                    }
                    Row::DiffLine { file_idx, line_idx } => {
                        let file = &state.files[file_idx];
                        let line = &file.lines[line_idx];

                        let is_active = active_line
                            .map(|ctx| ctx.file_idx == file_idx && ctx.line_idx == line_idx)
                            .unwrap_or(false);

                        let line_action = render_unified_row(
                            ui,
                            line,
                            LineContext { file_idx, line_idx },
                            is_active,
                            on_comment_requested,
                            active_line,
                        );

                        if let DiffAction::None = action {
                            action = line_action;
                        }
                    }
                }
            }
        });

    ui.ctx()
        .memory_mut(|mem| mem.data.insert_persisted(state_id, state.clone()));

    if let DiffAction::OpenFullWindow = action {
        action
    } else if open_full {
        DiffAction::OpenFullWindow
    } else {
        action
    }
}

fn middle_truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    let keep = (max_len - 3) / 2;
    let start = &s[..keep];
    let end = &s[s.len() - keep..];
    format!("{}...{}", start, end)
}

fn render_file_header(
    ui: &mut egui::Ui,
    file_idx: usize,
    state: &mut DiffState,
    state_id: egui::Id,
) {
    let theme = theme::current_theme();
    let file = &state.files[file_idx];

    let is_open = !state.collapsed[file_idx];
    let icon_closed = egui_phosphor::regular::PLUS;
    let icon_open = egui_phosphor::regular::MINUS;
    let icon = if is_open { icon_open } else { icon_closed };

    let clicked = ui
        .scope(|ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            ui.horizontal(|ui| {
                let mut clicked_local = false;

                let display_path = if file.new_path != "/dev/null" {
                    &file.new_path
                } else {
                    &file.old_path
                };

                // 1. Draw Button (Left)
                if ui
                    .button(
                        egui::RichText::new(icon.to_string())
                            .size(DIFF_FONT_SIZE)
                            .color(theme.text_primary),
                    )
                    .clicked()
                {
                    clicked_local = true;
                }

                // 2. Fill remainder (Path Left, Stats Right)

                // Calculate reserved space for stats
                let mut stats_reserved_width = 0.0;
                if file.additions > 0 {
                    stats_reserved_width += 60.0;
                }
                if file.deletions > 0 {
                    stats_reserved_width += 60.0;
                }

                // Draw Path (Left Aligned)
                let available_width = ui.available_width();
                let path_area_width = (available_width - stats_reserved_width).max(50.0);

                let char_capacity = (path_area_width / 7.0) as usize;
                let max_len = char_capacity.saturating_sub(3).max(15);

                let truncated_path = middle_truncate(display_path, max_len);

                ui.label(
                    egui::RichText::new(truncated_path)
                        .strong()
                        .color(theme.text_primary)
                        .size(HEADER_FONT_SIZE),
                )
                .on_hover_text(display_path);

                // Draw Stats (Right Aligned)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if file.additions > 0 {
                        ui.label(
                            egui::RichText::new(format!("+{}", file.additions))
                                .color(theme.success)
                                .size(DIFF_FONT_SIZE),
                        );
                    }
                    if file.deletions > 0 {
                        ui.label(
                            egui::RichText::new(format!("-{}", file.deletions))
                                .color(theme.destructive)
                                .size(DIFF_FONT_SIZE),
                        );
                    }
                });

                clicked_local
            })
            .inner
        })
        .inner;

    ui.add_space(2.0);
    ui.separator();

    if clicked {
        state.collapsed[file_idx] = !state.collapsed[file_idx];

        let (rows, row_height) = build_row_list(&state.files, &state.collapsed, ui);
        state.rows = rows;
        state.row_height = row_height;

        let scroll_id = ui.id().with("diff_scroll");
        ui.ctx().memory_mut(|mem| {
            mem.data.remove::<egui::scroll_area::State>(scroll_id);
        });

        ui.ctx()
            .memory_mut(|mem| mem.data.insert_persisted(state_id, state.clone()));
    }
}

fn render_unified_row(
    ui: &mut egui::Ui,
    line: &DiffLine,
    ctx: LineContext,
    is_active: bool,
    on_comment_click: Option<&dyn Fn(usize, usize, usize)>,
    _active_comment_line: Option<LineContext>,
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

                        if let Some(segments) = &line.inline_segments {
                            let highlight_bg = match line.change_type {
                                ChangeType::Delete => theme.destructive.gamma_multiply(0.4),
                                ChangeType::Insert => theme.success.gamma_multiply(0.4),
                                ChangeType::Equal => theme.transparent,
                            };
                            paint_inline_text_job(&mut job, segments, text_color, highlight_bg);
                        } else {
                            job.append(
                                line.content.as_ref(),
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
                .desired_rows(3)
                .desired_width(ui.available_width()),
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
