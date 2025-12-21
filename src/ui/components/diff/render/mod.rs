pub mod header;
pub mod row;
pub mod types;
pub mod utils;

use super::DiffAction;
use super::model::{ChangeType, DiffViewState, LineContext};
use crate::ui::components::diff::indexer::get_change_type_from_line;
use crate::ui::theme;
use eframe::egui;
use std::sync::Arc;
use types::{CachedLineInfo, DiffLineInfo, LineCache, LineCacheKey, RowType};
use utils::{
    calculate_total_rows, compute_inline_diff_if_appropriate, extract_line_numbers, get_row_type,
    strip_git_prefix,
};

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
    let state_id = ui.id().with("diff_view_state");

    let mut view_state = ui
        .ctx()
        .memory_mut(|mem| mem.data.get_persisted::<DiffViewState>(state_id))
        .unwrap_or_default();

    let new_hash = egui::util::hash(diff_text.as_bytes());
    let diff_changed = view_state.last_hash != new_hash;

    let doc = Arc::new(super::indexer::index_diff(diff_text));

    if diff_changed {
        view_state.last_hash = new_hash;
        view_state.parse_error = None;
        view_state.collapsed = vec![false; doc.files.len()];

        ui.ctx()
            .memory_mut(|mem| mem.data.insert_persisted(state_id, view_state.clone()));
    }

    if let Some(err) = &view_state.parse_error {
        let msg = format!("{} {}", egui_phosphor::regular::WARNING, err);
        let theme = theme::current_theme();
        ui.colored_label(theme.destructive, msg);
        return DiffAction::None;
    }

    let mut open_full = false;
    let theme = theme::current_theme();

    let total_additions: u32 = doc.files.iter().map(|f| f.additions).sum();
    let total_deletions: u32 = doc.files.iter().map(|f| f.deletions).sum();
    let all_collapsed = !view_state.collapsed.is_empty() && view_state.collapsed.iter().all(|&c| c);

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Diff").color(theme.text_primary));
        ui.label(egui::RichText::new(format!(
            "{} {} files",
            egui_phosphor::regular::FILES,
            doc.files.len()
        )));

        if total_additions > 0 {
            ui.label(
                egui::RichText::new(format!("+{}", total_additions))
                    .color(theme.success)
                    .size(utils::DIFF_FONT_SIZE),
            );
        }
        if total_deletions > 0 {
            ui.label(
                egui::RichText::new(format!("-{}", total_deletions))
                    .color(theme.destructive)
                    .size(utils::DIFF_FONT_SIZE),
            );
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if show_full_window_button
                && ui
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

            let toggle_icon = if all_collapsed {
                egui_phosphor::regular::ARROWS_OUT_SIMPLE
            } else {
                egui_phosphor::regular::ARROWS_IN_SIMPLE
            };
            let toggle_text = if all_collapsed {
                "Expand All"
            } else {
                "Collapse All"
            };

            if ui
                .button(
                    egui::RichText::new(format!("{} {}", toggle_icon, toggle_text))
                        .color(theme.text_secondary),
                )
                .clicked()
            {
                let new_state = !all_collapsed;
                view_state.collapsed = vec![new_state; doc.files.len()];
                ui.ctx()
                    .memory_mut(|mem| mem.data.insert_persisted(state_id, view_state.clone()));
            }
        });
    });

    ui.add_space(4.0);

    let total_rows = calculate_total_rows(&doc, &view_state.collapsed);
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 2.0;

    let cache_id = ui.id().with("line_cache");
    let mut cache = if diff_changed {
        LineCache::new(2000)
    } else {
        ui.ctx()
            .memory_mut(|mem| mem.data.get_temp::<LineCache>(cache_id))
            .unwrap_or_else(|| LineCache::new(2000))
    };

    let overscan_before = 200;
    let overscan_after = 400;

    egui::ScrollArea::vertical()
        .id_salt("diff_scroll")
        .auto_shrink([false; 2])
        .show_rows(ui, row_height, total_rows, |ui, range| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);

            let overscan_start = range.start.saturating_sub(overscan_before);
            let overscan_end = (range.end + overscan_after).min(total_rows);
            let overscan_range = overscan_start..overscan_end;

            for idx in overscan_range {
                if let Some(RowType::DiffLine { file_idx, line_idx }) =
                    get_row_type(&doc, &view_state.collapsed, idx)
                {
                    let cache_key = LineCacheKey { file_idx, line_idx };
                    if !cache.cache.contains_key(&cache_key) {
                        let line_str = doc.line_str(line_idx);
                        let change_type = get_change_type_from_line(line_str);
                        let (old_line_num, new_line_num) =
                            extract_line_numbers(&doc, file_idx, line_idx);

                        let mut inline_segments = None;
                        if !matches!(change_type, ChangeType::Equal) {
                            let segments =
                                compute_inline_diff_if_appropriate(line_str, &change_type);
                            inline_segments = Some(segments);
                        }

                        let cached_info = CachedLineInfo {
                            old_line_num,
                            new_line_num,
                            content: line_str.to_string(),
                            change_type,
                            inline_segments,
                        };
                        cache.insert(cache_key, cached_info);
                    } else if let Some(cached) = cache.get(&cache_key)
                        && cached.inline_segments.is_none()
                        && idx >= range.start
                        && idx < range.end
                    {
                        let line_str = doc.line_str(line_idx);
                        let change_type = cached.change_type;
                        let segments = compute_inline_diff_if_appropriate(line_str, &change_type);
                        let updated_cached_info = CachedLineInfo {
                            old_line_num: cached.old_line_num,
                            new_line_num: cached.new_line_num,
                            content: cached.content.clone(),
                            change_type: cached.change_type,
                            inline_segments: Some(segments),
                        };
                        cache.insert(cache_key, updated_cached_info);
                    }
                }
            }

            for idx in range {
                if let Some(row_type) = get_row_type(&doc, &view_state.collapsed, idx) {
                    match row_type {
                        RowType::FileHeader { file_idx } => {
                            header::render_file_header(
                                ui,
                                file_idx,
                                &mut view_state,
                                state_id,
                                &doc,
                            );
                        }
                        RowType::DiffLine { file_idx, line_idx } => {
                            let cache_key = LineCacheKey { file_idx, line_idx };
                            let diff_line_info = if let Some(cached) = cache.get(&cache_key) {
                                DiffLineInfo {
                                    old_line_num: cached.old_line_num,
                                    new_line_num: cached.new_line_num,
                                    content: cached.content.clone(),
                                    change_type: cached.change_type,
                                    inline_segments: cached.inline_segments.clone(),
                                }
                            } else {
                                let line_str = doc.line_str(line_idx);
                                let change_type = get_change_type_from_line(line_str);
                                let (old_line_num, new_line_num) =
                                    extract_line_numbers(&doc, file_idx, line_idx);
                                let segments =
                                    compute_inline_diff_if_appropriate(line_str, &change_type);
                                DiffLineInfo {
                                    old_line_num,
                                    new_line_num,
                                    content: line_str.to_string(),
                                    change_type,
                                    inline_segments: Some(segments),
                                }
                            };

                            let file = &doc.files[file_idx];
                            let file_path = if file.new_path != "/dev/null" {
                                strip_git_prefix(&file.new_path)
                            } else {
                                strip_git_prefix(&file.old_path)
                            };

                            let ctx = LineContext {
                                file_idx,
                                line_idx: line_idx as usize,
                                file_path,
                            };

                            let is_active = active_line
                                .as_ref()
                                .map(|ctx| {
                                    ctx.file_idx == file_idx && ctx.line_idx == line_idx as usize
                                })
                                .unwrap_or(false);

                            let line_action = row::render_unified_row(
                                ui,
                                &diff_line_info,
                                ctx,
                                is_active,
                                on_comment_requested,
                            );

                            if let DiffAction::None = action {
                                action = line_action;
                            }
                        }
                    }
                }
            }
        });

    ui.ctx()
        .memory_mut(|mem| mem.data.insert_temp(cache_id, cache));
    ui.ctx()
        .memory_mut(|mem| mem.data.insert_persisted(state_id, view_state));

    if let DiffAction::OpenFullWindow = action {
        action
    } else if open_full {
        DiffAction::OpenFullWindow
    } else {
        action
    }
}
