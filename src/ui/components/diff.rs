//! Unified diff display component for LaReview
//! Handles parsing, rendering, and interaction with git diffs in a unified format
//! with syntax highlighting, inline diffs, and collapsible file sections.

use catppuccin_egui::MOCHA;
use eframe::egui::{self, FontId, TextFormat, text::LayoutJob};
use similar::{ChangeTag, TextDiff};
use std::sync::Arc;
use unidiff::{Hunk, PatchSet, Result as UnidiffResult};

/// Possible actions that can be triggered from the diff viewer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffAction {
    /// No action was triggered
    None,
    /// Open the diff in full window view
    OpenFullWindow,
}

const DIFF_FONT_SIZE: f32 = 12.0;
const HEADER_FONT_SIZE: f32 = 14.0;

// Inline diff thresholds
const MAX_INLINE_LEN: usize = 600;

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
    render_diff_editor_with_options(ui, diff_text, _language, true)
}

pub fn render_diff_editor_full_view(
    ui: &mut egui::Ui,
    diff_text: &str,
    _language: &str,
) -> DiffAction {
    render_diff_editor_with_options(ui, diff_text, _language, false)
}

fn render_diff_editor_with_options(
    ui: &mut egui::Ui,
    diff_text: &str,
    _language: &str,
    show_full_window_button: bool,
) -> DiffAction {
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
                        render_unified_row(ui, line);
                    }
                }
            }
        });

    ui.ctx()
        .memory_mut(|mem| mem.data.insert_persisted(state_id, state.clone()));

    if open_full {
        DiffAction::OpenFullWindow
    } else {
        DiffAction::None
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
fn render_unified_row(ui: &mut egui::Ui, line: &DiffLine) {
    let (prefix, bg_color, text_color, line_num_bg) = match line.change_type {
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

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;

        // Line number section with darker background
        egui::Frame::NONE
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

        // Content section
        egui::Frame::NONE
            .fill(bg_color)
            .inner_margin(egui::Margin::symmetric(4, 0))
            .show(ui, |ui| {
                let mut job = LayoutJob::default();

                // Add prefix
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

                // Add content with inline highlighting if available
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
}
