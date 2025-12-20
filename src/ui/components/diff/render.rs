use std::sync::Arc;

use super::DiffAction;
use super::{
    doc::DiffDoc,
    indexer::get_change_type_from_line,
    model::{ChangeType, DiffViewState, LineContext},
};
use crate::ui::spacing;
use crate::ui::theme;
use eframe::egui::{self, FontId, TextFormat, text::LayoutJob};
use egui_phosphor::regular::PLUS;

const DIFF_FONT_SIZE: f32 = 12.0;
const HEADER_FONT_SIZE: f32 = 14.0;

// Define row types for the new architecture
#[derive(Debug, Clone)]
enum RowType {
    FileHeader { file_idx: usize },
    DiffLine { file_idx: usize, line_idx: u32 }, // line_idx is the global line index in the diff doc
}

// Function to calculate row mapping on demand
fn get_row_type(doc: &DiffDoc, collapsed: &[bool], row_idx: usize) -> Option<RowType> {
    let mut current_row = 0;

    for (file_idx, file) in doc.files.iter().enumerate() {
        // Add file header row
        if current_row == row_idx {
            return Some(RowType::FileHeader { file_idx });
        }
        current_row += 1;

        // Add diff lines if file is expanded
        if file_idx < collapsed.len() && !collapsed[file_idx] {
            for line_idx in file.line_range.clone() {
                // Skip header lines that shouldn't be rendered as diff content
                let line_str = doc.line_str(line_idx);
                if should_render_line(line_str) {
                    if current_row == row_idx {
                        return Some(RowType::DiffLine { file_idx, line_idx });
                    }
                    current_row += 1;
                }
            }
        }
    }

    None
}

// Calculate total number of rows (headers + expanded lines)
fn calculate_total_rows(doc: &DiffDoc, collapsed: &[bool]) -> usize {
    let mut total = 0;
    for (file_idx, file) in doc.files.iter().enumerate() {
        total += 1; // file header

        // Add lines only if file is not collapsed
        if file_idx < collapsed.len() && !collapsed[file_idx] {
            for line_idx in file.line_range.clone() {
                let line_str = doc.line_str(line_idx);
                if should_render_line(line_str) {
                    total += 1;
                }
            }
        }
    }
    total
}

// Determine if a line should be rendered as part of the diff content
fn should_render_line(line: &str) -> bool {
    // Don't render these header lines as diff content:
    // - diff --git lines
    // - --- lines
    // - +++ lines
    // - @@ lines
    // - index lines
    !line.starts_with("diff --git ")
        && !line.starts_with("--- ")
        && !line.starts_with("+++ ")
        && !line.starts_with("@@ ")
        && !line.starts_with("index ")
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
    let state_id = ui.id().with("diff_view_state");

    // Load only the light UI state from egui memory
    let mut view_state = ui
        .ctx()
        .memory_mut(|mem| mem.data.get_persisted::<DiffViewState>(state_id))
        .unwrap_or_default();

    let new_hash = egui::util::hash(diff_text.as_bytes());

    // Create the DiffDoc (for now, keeping indexing on UI thread for simplicity)
    // In a complete implementation, indexing would happen in background
    let doc = Arc::new(super::indexer::index_diff(diff_text));

    if view_state.last_hash != new_hash {
        // Reset UI state for new diff
        view_state.last_hash = new_hash;
        view_state.parse_error = None;

        // Initialize collapsed state for all files
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

    // Calculate total stats
    let total_additions: u32 = doc.files.iter().map(|f| f.additions).sum();
    let total_deletions: u32 = doc.files.iter().map(|f| f.deletions).sum();

    // Determine collapse/expand all state
    let all_collapsed = !view_state.collapsed.is_empty() && view_state.collapsed.iter().all(|&c| c);

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Diff").color(theme.text_primary));

        ui.label(egui::RichText::new(format!(
            "{} {} files",
            egui_phosphor::regular::FILES,
            doc.files.len()
        )));

        // Total stats
        if total_additions > 0 {
            ui.label(
                egui::RichText::new(format!("+{}", total_additions))
                    .color(theme.success)
                    .size(DIFF_FONT_SIZE),
            );
        }
        if total_deletions > 0 {
            ui.label(
                egui::RichText::new(format!("-{}", total_deletions))
                    .color(theme.destructive)
                    .size(DIFF_FONT_SIZE),
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

            // Collapse/Expand All Button
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

    // Calculate the total number of rows once
    let total_rows = calculate_total_rows(&doc, &view_state.collapsed);
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 2.0;

    // Set up caches
    let cache_id = ui.id().with("line_cache");
    let mut cache = ui
        .ctx()
        .memory_mut(|mem| mem.data.get_temp::<LineCache>(cache_id))
        .unwrap_or_else(|| LineCache::new(2000)); // Cache up to 2000 lines

    // Calculate overscan range
    // In a real implementation, this would be computed from the scroll area state
    // For now, we'll compute it based on the range that show_rows receives
    let overscan_before = 200; // Lines to prefetch before visible area
    let overscan_after = 400; // Lines to prefetch after visible area

    egui::ScrollArea::vertical()
        .id_salt("diff_scroll")
        .auto_shrink([false; 2])
        .show_rows(ui, row_height, total_rows, |ui, range| {
            // Set truncation mode for consistent row height
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);

            // Calculate the overscan range
            let overscan_start = range.start.saturating_sub(overscan_before);
            let overscan_end = (range.end + overscan_after).min(total_rows);
            let overscan_range = overscan_start..overscan_end;

            // Prefetch/cache data for overscan range
            for idx in overscan_range {
                if let Some(RowType::DiffLine { file_idx, line_idx }) =
                    get_row_type(&doc, &view_state.collapsed, idx)
                {
                    // Check if we already have this in cache
                    let cache_key = LineCacheKey { file_idx, line_idx };
                    if !cache.cache.contains_key(&cache_key) {
                        // Compute and cache the line information
                        let line_str = doc.line_str(line_idx);
                        let change_type = get_change_type_from_line(line_str);
                        let (old_line_num, new_line_num) =
                            extract_line_numbers(&doc, file_idx, line_idx);

                        // Compute inline diff for visible and overscan rows (lazy computation)
                        let mut inline_segments = None;
                        if matches!(change_type, ChangeType::Equal) {
                            // For equal lines, we might want to compute inline diff if they're similar but not identical
                            // For now, we'll compute inline diffs for all lines in overscan range
                            if let (Some(_old_num), Some(_new_num)) = (old_line_num, new_line_num) {
                                // Get the old and new content for inline diff
                                // In a real implementation, we would retrieve the corresponding lines
                                // But for now, we'll just compute based on the current line content
                                if change_type == ChangeType::Equal {
                                    // For context lines, there's no inline diff to compute
                                } else {
                                    // For insert/delete lines, we can potentially compute inline diffs
                                    // by finding corresponding old/new lines
                                    let segments = compute_inline_diff_if_appropriate(
                                        line_str,
                                        &doc,
                                        file_idx,
                                        line_idx,
                                        &change_type,
                                    );
                                    inline_segments = Some(segments);
                                }
                            }
                        }

                        let cached_info = CachedLineInfo {
                            old_line_num,
                            new_line_num,
                            content: line_str.to_string(),
                            change_type,
                            inline_segments,
                        };

                        cache.insert(cache_key, cached_info);
                    } else if let Some(cached) = cache.get(&cache_key) {
                        // If the item is already cached but doesn't have inline segments computed,
                        // and we're in the visible range, compute it now
                        if cached.inline_segments.is_none() && idx >= range.start && idx < range.end
                        {
                            // Update the cache with computed inline segments
                            let line_str = doc.line_str(line_idx);
                            let change_type = cached.change_type;

                            let segments = compute_inline_diff_if_appropriate(
                                line_str,
                                &doc,
                                file_idx,
                                line_idx,
                                &change_type,
                            );

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
            }

            // Now render the actual visible range
            for idx in range {
                if let Some(row_type) = get_row_type(&doc, &view_state.collapsed, idx) {
                    match row_type {
                        RowType::FileHeader { file_idx } => {
                            render_file_header(ui, file_idx, &mut view_state, state_id, &doc);
                        }
                        RowType::DiffLine { file_idx, line_idx } => {
                            // Try to get from cache first, fall back to computation if needed
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
                                // Fallback to on-demand computation
                                let line_str = doc.line_str(line_idx);
                                let change_type = get_change_type_from_line(line_str);
                                let (old_line_num, new_line_num) =
                                    extract_line_numbers(&doc, file_idx, line_idx);

                                // Compute inline diff for currently rendering line
                                let segments = compute_inline_diff_if_appropriate(
                                    line_str,
                                    &doc,
                                    file_idx,
                                    line_idx,
                                    &change_type,
                                );

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

                            let line_action = render_unified_row(
                                ui,
                                &diff_line_info,
                                ctx,
                                is_active,
                                on_comment_requested,
                                active_line.clone(),
                            );

                            if let DiffAction::None = action {
                                action = line_action;
                            }
                        }
                    }
                }
            }
        });

    // Store the cache in temporary memory
    ui.ctx()
        .memory_mut(|mem| mem.data.insert_temp(cache_id, cache));

    // Only store the lightweight view state in egui memory
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

fn extract_line_numbers(
    doc: &DiffDoc,
    file_idx: usize,
    line_idx: u32,
) -> (Option<usize>, Option<usize>) {
    // This calls the indexer function to accurately calculate line numbers
    let (old_line, new_line) = super::indexer::calculate_line_numbers(doc, file_idx, line_idx);
    (old_line.map(|x| x as usize), new_line.map(|x| x as usize))
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
    view_state: &mut DiffViewState,
    state_id: egui::Id,
    doc: &DiffDoc,
) {
    let theme = theme::current_theme();
    let file = &doc.files[file_idx];

    let is_open = if file_idx < view_state.collapsed.len() {
        !view_state.collapsed[file_idx]
    } else {
        false
    };
    let icon_closed = egui_phosphor::regular::PLUS;
    let icon_open = egui_phosphor::regular::MINUS;
    let icon = if is_open { icon_open } else { icon_closed };

    let clicked = ui
        .scope(|ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            ui.horizontal(|ui| {
                let mut clicked_local = false;

                // Remove "a/" and "b/" prefixes from paths
                let display_path = if file.new_path != "/dev/null" {
                    strip_git_prefix(&file.new_path)
                } else {
                    strip_git_prefix(&file.old_path)
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

                let truncated_path = middle_truncate(&display_path, max_len);

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
        if file_idx < view_state.collapsed.len() {
            view_state.collapsed[file_idx] = !view_state.collapsed[file_idx];
        } else {
            // Extend the collapsed vector if needed
            while view_state.collapsed.len() <= file_idx {
                view_state.collapsed.push(false);
            }
            view_state.collapsed[file_idx] = true;
        }

        let scroll_id = ui.id().with("diff_scroll");
        ui.ctx().memory_mut(|mem| {
            mem.data.remove::<egui::scroll_area::State>(scroll_id);
        });

        ui.ctx()
            .memory_mut(|mem| mem.data.insert_persisted(state_id, view_state.clone()));
    }
}

// Helper function to strip git prefixes from file paths
fn strip_git_prefix(path: &str) -> String {
    path.trim_start_matches("a/")
        .trim_start_matches("b/")
        .to_string()
}

use std::collections::HashMap;

// Cache key for precomputed line information
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct LineCacheKey {
    file_idx: usize,
    line_idx: u32,
}

// Cached information for a diff line
#[derive(Debug, Clone)]
struct CachedLineInfo {
    old_line_num: Option<usize>,
    new_line_num: Option<usize>,
    content: String,
    change_type: ChangeType,
    inline_segments: Option<Vec<(String, bool)>>, // (text, highlight)
                                                  // Additional layout info could be cached here
}

/// Compute inline diff for appropriate lines based on context
fn compute_inline_diff_if_appropriate(
    line_str: &str,
    _doc: &DiffDoc,
    _file_idx: usize,
    _line_idx: u32,
    change_type: &ChangeType,
) -> Vec<(String, bool)> {
    // Only compute inline diff for certain line types
    match change_type {
        ChangeType::Equal => {
            // For equal lines, we could compute inline diff against a reference,
            // but typically these don't need inline diffs
            vec![(line_str.to_string(), false)]
        }
        ChangeType::Delete => {
            // For delete lines, find corresponding insert line if possible
            // For now, return the content as deleted
            vec![(line_str.trim_start_matches('-').to_string(), true)]
        }
        ChangeType::Insert => {
            // For insert lines, find corresponding delete line if possible
            // For now, return the content as inserted
            vec![(line_str.trim_start_matches('+').to_string(), true)]
        }
    }
}

// Cache for precomputed line information
#[derive(Debug, Clone)]
struct LineCache {
    cache: HashMap<LineCacheKey, CachedLineInfo>,
    max_size: usize,
}

#[allow(dead_code)]
impl LineCache {
    fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
        }
    }

    fn get(&self, key: &LineCacheKey) -> Option<&CachedLineInfo> {
        self.cache.get(key)
    }

    fn insert(&mut self, key: LineCacheKey, value: CachedLineInfo) {
        if self.cache.len() >= self.max_size {
            // Simple eviction: remove a random entry if at capacity
            if let Some(first_key) = self.cache.keys().next().cloned() {
                self.cache.remove(&first_key);
            }
        }
        self.cache.insert(key, value);
    }

    fn clear(&mut self) {
        self.cache.clear();
    }
}

// Helper struct to represent diff line information for rendering
struct DiffLineInfo {
    old_line_num: Option<usize>,
    new_line_num: Option<usize>,
    content: String,
    change_type: ChangeType,
    inline_segments: Option<Vec<(String, bool)>>, // (text, highlight)
}

fn render_unified_row(
    ui: &mut egui::Ui,
    line: &DiffLineInfo,
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

                        // Truncate the content to avoid wrapping
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
                            // If no inline segments computed, just render the content as regular text
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
