//! Unified diff display component for LaReview
//! Handles parsing, rendering, and interaction with git diffs in a unified format
//! with syntax highlighting, inline diffs, and collapsible file sections.

use catppuccin_egui::MOCHA;
use eframe::egui::{self, FontId, TextFormat, text::LayoutJob};
use egui_phosphor::regular::CHAT_DOTS;
use similar::{ChangeTag, TextDiff};
use std::sync::Arc;
use unidiff::{Hunk, PatchSet, Result as UnidiffResult}; // Import the desired icon

/// Possible actions that can be triggered from the diff viewer
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffAction {
    /// No action was triggered
    None,
    /// Open the diff in full window view
    OpenFullWindow,
    /// A line was clicked for commenting.
    /// Carries the 0-based index of the line in the FileDiff structure, and the
    /// line number in the source file (old_line_num or new_line_num).
    AddNote {
        file_idx: usize,
        line_idx: usize,
        line_number: usize,
    },
    /// Save a note for a line
    SaveNote {
        file_idx: usize,
        line_idx: usize,
        line_number: usize,
        note_text: String,
    },
}

const DIFF_FONT_SIZE: f32 = 12.0;
const HEADER_FONT_SIZE: f32 = 14.0;

// Inline diff thresholds
const MAX_INLINE_LEN: usize = 600;

// ADDED: New struct to pass context for note addition
#[derive(Debug, Clone, Copy)]
pub struct LineContext {
    pub file_idx: usize,
    pub line_idx: usize,
}

fn should_do_inline(old: &str, new: &str) -> bool {
    old.len() <= MAX_INLINE_LEN && new.len() <= MAX_INLINE_LEN
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ChangeType {
    Equal,
    Delete,
    Insert,
}

#[derive(Debug, Clone)]
struct DiffLine {
    old_line_num: Option<usize>,
    new_line_num: Option<usize>,
    content: Arc<str>,
    change_type: ChangeType,
    inline_segments: Option<Vec<(String, bool)>>, // (text, highlight)
}

#[derive(Debug, Clone)]
struct FileDiff {
    old_path: String,
    new_path: String,
    lines: Vec<DiffLine>,
    additions: usize,
    deletions: usize,
}

#[derive(Default, Clone)]
struct DiffState {
    last_hash: u64,
    files: Vec<FileDiff>,
    parse_error: Option<String>,
    rows: Vec<Row>,
    row_height: f32,
    collapsed: Vec<bool>, // Per-file collapse state
}

#[derive(Clone)]
enum Row {
    FileHeader { file_idx: usize },
    DiffLine { file_idx: usize, line_idx: usize },
}

fn strip_git_prefix(path: &str) -> String {
    path.trim_start_matches("a/")
        .trim_start_matches("b/")
        .to_string()
}

// Inline diff segmentation - returns segments for a single line
fn inline_segments(old: &str, new: &str) -> Vec<(String, bool)> {
    let diff = TextDiff::from_chars(old, new);
    let mut segments = Vec::new();

    for change in diff.iter_all_changes() {
        let text = change.value().to_string();
        match change.tag() {
            ChangeTag::Equal => segments.push((text, false)),
            ChangeTag::Delete | ChangeTag::Insert => segments.push((text, true)),
        }
    }

    segments
}

// Build internal line representation - GitHub style
fn build_lines_for_hunk(hunk: &Hunk, out: &mut Vec<DiffLine>) {
    let lines = hunk.lines();
    let mut i = 0usize;

    while i < lines.len() {
        let line = &lines[i];

        // Context line
        if line.is_context() {
            let arc: Arc<str> = Arc::from(line.value.as_str());
            out.push(DiffLine {
                old_line_num: line.source_line_no,
                new_line_num: line.target_line_no,
                content: arc,
                change_type: ChangeType::Equal,
                inline_segments: None,
            });
            i += 1;
            continue;
        }

        // Find the block of removals and additions
        let remove_start = i;
        let mut j = i;

        while j < lines.len() && lines[j].is_removed() {
            j += 1;
        }
        let insert_start = j;

        while j < lines.len() && lines[j].is_added() {
            j += 1;
        }

        let removed = &lines[remove_start..insert_start];
        let added = &lines[insert_start..j];

        // GitHub-style: try to align similar lines
        if !removed.is_empty() && !added.is_empty() && removed.len() == added.len() {
            // When we have equal numbers, try to pair them up with inline diffs
            let mut has_similar = false;

            for (old_line, new_line) in removed.iter().zip(added.iter()) {
                let old_text = old_line.value.as_str();
                let new_text = new_line.value.as_str();

                // Check if lines are similar enough to show inline diff
                let similarity = similar::TextDiff::from_chars(old_text, new_text).ratio();

                if similarity > 0.3 {
                    has_similar = true;
                    break;
                }
            }

            if has_similar {
                // Pair them up with inline diffs
                for (old_line, new_line) in removed.iter().zip(added.iter()) {
                    let old_text = old_line.value.as_str();
                    let new_text = new_line.value.as_str();

                    let similarity = similar::TextDiff::from_chars(old_text, new_text).ratio();

                    if similarity > 0.3 && should_do_inline(old_text, new_text) {
                        // Show as deletion with inline diff
                        out.push(DiffLine {
                            old_line_num: old_line.source_line_no,
                            new_line_num: None,
                            content: Arc::from(old_text),
                            change_type: ChangeType::Delete,
                            inline_segments: Some(inline_segments(old_text, new_text)),
                        });

                        // Show as insertion with inline diff
                        out.push(DiffLine {
                            old_line_num: None,
                            new_line_num: new_line.target_line_no,
                            content: Arc::from(new_text),
                            change_type: ChangeType::Insert,
                            inline_segments: Some(inline_segments(new_text, old_text)),
                        });
                    } else {
                        // Not similar enough, show as separate delete/insert
                        out.push(DiffLine {
                            old_line_num: old_line.source_line_no,
                            new_line_num: None,
                            content: Arc::from(old_text),
                            change_type: ChangeType::Delete,
                            inline_segments: None,
                        });

                        out.push(DiffLine {
                            old_line_num: None,
                            new_line_num: new_line.target_line_no,
                            content: Arc::from(new_text),
                            change_type: ChangeType::Insert,
                            inline_segments: None,
                        });
                    }
                }

                i = j;
                continue;
            }
        }

        // Fallback: show all deletions, then all insertions (GitHub style)
        for old_line in removed {
            out.push(DiffLine {
                old_line_num: old_line.source_line_no,
                new_line_num: None,
                content: Arc::from(old_line.value.as_str()),
                change_type: ChangeType::Delete,
                inline_segments: None,
            });
        }

        for new_line in added {
            out.push(DiffLine {
                old_line_num: None,
                new_line_num: new_line.target_line_no,
                content: Arc::from(new_line.value.as_str()),
                change_type: ChangeType::Insert,
                inline_segments: None,
            });
        }

        i = j;
    }
}

// Parse whole diff
fn parse_diff_by_files(diff_text: &str) -> UnidiffResult<Vec<FileDiff>> {
    let trimmed = diff_text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let mut patch = PatchSet::new();
    patch.parse(trimmed)?;

    let mut files_out = Vec::new();

    for file in patch.files() {
        let mut lines = Vec::new();

        for hunk in file.hunks() {
            build_lines_for_hunk(hunk, &mut lines);
        }

        files_out.push(FileDiff {
            old_path: strip_git_prefix(&file.source_file),
            new_path: strip_git_prefix(&file.target_file),
            lines,
            additions: file.added(),
            deletions: file.removed(),
        });
    }

    Ok(files_out)
}

// Build virtual row list based on collapse state
fn build_row_list(files: &[FileDiff], collapsed: &[bool], ui: &egui::Ui) -> (Vec<Row>, f32) {
    let mut rows = Vec::new();

    // Monospace height plus 2px padding for clean GitHub look
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

// Helper to append inline segments to a LayoutJob
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

pub fn render_diff_editor(ui: &mut egui::Ui, diff_text: &str, _language: &str) -> DiffAction {
    render_diff_editor_with_options(ui, diff_text, _language, true, None, None)
}

pub fn render_diff_editor_full_view(
    ui: &mut egui::Ui,
    diff_text: &str,
    _language: &str,
) -> DiffAction {
    render_diff_editor_with_options(ui, diff_text, _language, false, None, None)
}

pub fn render_diff_editor_with_comment_callback(
    ui: &mut egui::Ui,
    diff_text: &str,
    _language: &str,
    show_full_window_button: bool,
    active_line: Option<LineContext>,
    on_comment_requested: Option<&dyn Fn(usize, usize, usize)>,
) -> DiffAction {
    render_diff_editor_with_options(
        ui,
        diff_text,
        _language,
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
    // ADDED: Optional context to highlight the line that is being commented on
    active_line: Option<LineContext>,
    // ADDED: Optional callback for when a comment is requested for a line
    on_comment_requested: Option<&dyn Fn(usize, usize, usize)>, // file_idx, line_idx, line_number
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
                state.collapsed = vec![false; file_count];

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
        ui.colored_label(MOCHA.red, msg);
        return DiffAction::None;
    }

    let mut open_full = false;

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Diff").color(MOCHA.text));

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
                        .color(MOCHA.mauve),
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

                        // Check if this is the active line
                        let is_active = active_line
                            .map(|ctx| ctx.file_idx == file_idx && ctx.line_idx == line_idx)
                            .unwrap_or(false);

                        // Call the updated render function with the callbacks
                        let line_action = render_unified_row(
                            ui,
                            line,
                            LineContext { file_idx, line_idx },
                            is_active,
                            on_comment_requested,
                            active_line, // This should be the active comment line
                        );

                        // Only update action if it's not None
                        if let DiffAction::None = action {
                            action = line_action;
                        }
                    }
                }
            }
        });

    ui.ctx()
        .memory_mut(|mem| mem.data.insert_persisted(state_id, state.clone()));

    // Return the specific action if one was triggered
    if let DiffAction::OpenFullWindow = action {
        action
    } else if open_full {
        DiffAction::OpenFullWindow
    } else {
        action // Will be DiffAction::None or DiffAction::AddNote
    }
}

// Draw collapsible file header
fn render_file_header(
    ui: &mut egui::Ui,
    file_idx: usize,
    state: &mut DiffState,
    state_id: egui::Id,
) {
    let file = &state.files[file_idx];

    let is_open = !state.collapsed[file_idx];

    // Pick the icons you want
    let icon_closed = egui_phosphor::regular::PLUS;
    let icon_open = egui_phosphor::regular::MINUS;
    let icon = if is_open { icon_open } else { icon_closed };

    let clicked = ui
        .horizontal(|ui| {
            let mut clicked_local = false;

            let display_path = if file.new_path != "/dev/null" {
                &file.new_path
            } else {
                &file.old_path
            };

            // Icon button
            if ui
                .button(
                    egui::RichText::new(icon.to_string())
                        .size(DIFF_FONT_SIZE)
                        .color(MOCHA.text),
                )
                .clicked()
            {
                clicked_local = true;
            }

            // Path
            ui.label(
                egui::RichText::new(display_path)
                    .strong()
                    .color(MOCHA.text)
                    .size(HEADER_FONT_SIZE),
            );

            // Additions and deletions
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if file.deletions > 0 {
                    ui.label(
                        egui::RichText::new(format!("-{}", file.deletions))
                            .color(MOCHA.red)
                            .size(DIFF_FONT_SIZE),
                    );
                }
                if file.additions > 0 {
                    ui.label(
                        egui::RichText::new(format!("+{}", file.additions))
                            .color(MOCHA.green)
                            .size(DIFF_FONT_SIZE),
                    );
                }
            });

            clicked_local
        })
        .inner;

    ui.add_space(2.0);
    ui.separator();

    if clicked {
        state.collapsed[file_idx] = !state.collapsed[file_idx];

        let (rows, row_height) = build_row_list(&state.files, &state.collapsed, ui);
        state.rows = rows;
        state.row_height = row_height;

        // Clear scroll state so show_rows does not request stale indices
        let scroll_id = ui.id().with("diff_scroll");
        ui.ctx().memory_mut(|mem| {
            mem.data.remove::<egui::scroll_area::State>(scroll_id);
        });

        ui.ctx()
            .memory_mut(|mem| mem.data.insert_persisted(state_id, state.clone()));
    }
}

// Single unified diff line row - GitHub style
fn render_unified_row(
    ui: &mut egui::Ui,
    line: &DiffLine,
    ctx: LineContext,
    is_active: bool,
    // Optional callback for when the comment button is clicked
    on_comment_click: Option<&dyn Fn(usize, usize, usize)>, // file_idx, line_idx, line_number
    // No longer used for controlling which note is open, but kept for API compatibility
    _active_comment_line: Option<LineContext>,
) -> DiffAction {
    let mut action = DiffAction::None;

    // Per-line flag that tells us if the comment editor is open for this line
    let comment_open_id = ui.id().with(("comment_open", ctx.file_idx, ctx.line_idx));
    let mut is_comment_active =
        ui.memory(|mem| mem.data.get_temp::<bool>(comment_open_id).unwrap_or(false));

    // Actual line number in the source file
    let line_number = match line.change_type {
        ChangeType::Equal | ChangeType::Delete => line.old_line_num,
        ChangeType::Insert => line.new_line_num,
    };

    // Colors for this row
    let (prefix, mut bg_color, text_color, mut line_num_bg) = match line.change_type {
        ChangeType::Equal => (
            " ",
            egui::Color32::TRANSPARENT,
            MOCHA.text,
            egui::Color32::TRANSPARENT,
        ),
        ChangeType::Delete => (
            "-",
            MOCHA.red.gamma_multiply(0.15),
            MOCHA.red,
            MOCHA.red.gamma_multiply(0.25),
        ),
        ChangeType::Insert => (
            "+",
            MOCHA.green.gamma_multiply(0.15),
            MOCHA.green,
            MOCHA.green.gamma_multiply(0.25),
        ),
    };

    // Highlight whole line if it is "active" (selection etc)
    if is_active {
        let active_color = MOCHA.blue.gamma_multiply(0.2);
        bg_color = active_color;
        line_num_bg = active_color;
    }

    // Main row
    let main_response = egui::Frame::NONE
        .fill(bg_color)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.set_width(ui.available_width());

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;

                // Line numbers
                let _line_numbers_frame = egui::Frame::NONE
                    .fill(line_num_bg)
                    .inner_margin(egui::Margin::symmetric(4, 0))
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
                                .color(MOCHA.overlay0),
                        );
                    });

                // Code content
                egui::Frame::NONE
                    .fill(bg_color)
                    .inner_margin(egui::Margin::symmetric(4, 0))
                    .show(ui, |ui| {
                        let mut job = LayoutJob::default();

                        // Prefix (+/-/space)
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

                        // Inline content
                        if let Some(segments) = &line.inline_segments {
                            let highlight_bg = match line.change_type {
                                ChangeType::Delete => MOCHA.red.gamma_multiply(0.4),
                                ChangeType::Insert => MOCHA.green.gamma_multiply(0.4),
                                ChangeType::Equal => egui::Color32::TRANSPARENT,
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

    // Cursor feedback for the whole row
    if main_response.hovered() && line_number.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    let line_number_for_action = line_number;
    let mut comment_button_rect: Option<egui::Rect> = None;

    // Hover detection based on the actual row rect
    let pointer_pos = ui.ctx().input(|i| i.pointer.hover_pos());
    let is_line_hovered = pointer_pos.is_some_and(|p| main_response.rect.contains(p));

    // Only show the button when:
    // - we have a real line number
    // - this line does not already have an open note
    // - the pointer is over the row
    let show_comment_button = line_number.is_some() && !is_comment_active && is_line_hovered;

    if show_comment_button {
        let row_rect = main_response.rect;

        // Fixed visual size, independent of row height
        let size: f32 = 18.0;
        let button_size = egui::vec2(size, size);

        // Left gutter offset
        let offset_x: f32 = -4.0;

        // Vertically center in the row, with tiny tweak for optical centering
        let top_y = row_rect.center().y - size * 0.5 - 1.0;

        let button_rect =
            egui::Rect::from_min_size(egui::pos2(row_rect.left() + offset_x, top_y), button_size);

        // Save for cursor logic later
        comment_button_rect = Some(button_rect);

        let painter = ui.painter();

        // Background circle
        painter.rect_filled(
            button_rect,
            size * 0.5, // full rounding
            MOCHA.blue,
        );

        // Icon in the center
        painter.text(
            button_rect.center(),
            egui::Align2::CENTER_CENTER,
            CHAT_DOTS.to_string(),
            FontId::proportional(size - 4.0),
            MOCHA.base,
        );

        // Hover / click detection without affecting layout
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

    // Cursor feedback
    if let Some(pos) = pointer_pos {
        if let Some(btn_rect) = comment_button_rect {
            if btn_rect.contains(pos) {
                // pointer over the comment button
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            } else if main_response.rect.contains(pos) {
                // text selection cursor over the code line
                ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
            }
        } else if main_response.rect.contains(pos) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
        }
    }

    // Render the note editor under the line if it is open for this row
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

        // Keep the current text in memory
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

        // Optional: focus the editor the first time it is opened
        if text_response.gained_focus() {
            ui.ctx().memory_mut(|mem| {
                mem.request_focus(text_response.id);
            });
        }
    }

    action
}
