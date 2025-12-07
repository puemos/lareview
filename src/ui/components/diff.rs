//! Split view diff display (GitHub style) with collapsible files.
//!
//! Uses `unidiff` for parsing and `similar` for inline changes.

use super::theme::AppTheme;
use eframe::egui::{self, FontId, TextFormat, text::LayoutJob};
use similar::{ChangeTag, TextDiff};
use unidiff::{Hunk, PatchSet, Result as UnidiffResult};

#[derive(Debug, Clone, Copy, PartialEq)]
enum ChangeType {
    Equal,
    Delete,
    Insert,
    Replace,
}

#[derive(Debug, Clone)]
struct DiffLine {
    old_line_num: Option<usize>,
    new_line_num: Option<usize>,
    old_content: Option<String>,
    new_content: Option<String>,
    change_type: ChangeType,
}

#[derive(Debug, Clone)]
struct FileDiff {
    old_path: String,
    new_path: String,
    lines: Vec<DiffLine>,
    additions: usize,
    deletions: usize,
}

const DIFF_FONT_SIZE: f32 = 12.0;
const DIFF_HEADER_FONT_SIZE: f32 = 14.0;

fn build_lines_for_hunk(hunk: &Hunk, out: &mut Vec<DiffLine>) {
    let lines = hunk.lines();
    let mut i = 0usize;

    while i < lines.len() {
        let line = &lines[i];

        // Context line
        if line.is_context() {
            let text = line.value.clone();
            out.push(DiffLine {
                old_line_num: line.source_line_no,
                new_line_num: line.target_line_no,
                old_content: Some(text.clone()),
                new_content: Some(text),
                change_type: ChangeType::Equal,
            });
            i += 1;
            continue;
        }

        // Start of a change region: removed lines followed by added lines
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

        // Plain text slices for line level diff
        let old_block: Vec<&str> = removed.iter().map(|l| l.value.as_str()).collect();
        let new_block: Vec<&str> = added.iter().map(|l| l.value.as_str()).collect();

        let diff = TextDiff::from_slices(&old_block, &new_block);

        // Check if there is any op that has both old and new lines
        let has_mixed_op = diff
            .ops()
            .iter()
            .any(|op| !op.old_range().is_empty() && !op.new_range().is_empty());

        if has_mixed_op {
            // Use TextDiff alignment inside the block
            for op in diff.ops() {
                let old_range = op.old_range();
                let new_range = op.new_range();

                let left_len = old_range.len();
                let right_len = new_range.len();
                let max_len = std::cmp::max(left_len, right_len);

                for k in 0..max_len {
                    let left_line = if k < left_len {
                        Some(&removed[old_range.start + k])
                    } else {
                        None
                    };

                    let right_line = if k < right_len {
                        Some(&added[new_range.start + k])
                    } else {
                        None
                    };

                    let change_type = match (left_line, right_line) {
                        (Some(l), Some(r)) => {
                            if l.value == r.value {
                                ChangeType::Equal
                            } else {
                                ChangeType::Replace
                            }
                        }
                        (Some(_), None) => ChangeType::Delete,
                        (None, Some(_)) => ChangeType::Insert,
                        _ => unreachable!(),
                    };

                    out.push(DiffLine {
                        old_line_num: left_line.and_then(|l| l.source_line_no),
                        new_line_num: right_line.and_then(|r| r.target_line_no),
                        old_content: left_line.map(|l| l.value.clone()),
                        new_content: right_line.map(|r| r.value.clone()),
                        change_type,
                    });
                }
            }
        } else {
            // No overlap at all in this block
            // Fall back to pairing by index so big struct edits line up row by row
            let remove_count = removed.len();
            let insert_count = added.len();
            let row_count = std::cmp::max(remove_count, insert_count);

            for k in 0..row_count {
                let left_line = if k < remove_count {
                    Some(&removed[k])
                } else {
                    None
                };
                let right_line = if k < insert_count {
                    Some(&added[k])
                } else {
                    None
                };

                let change_type = match (left_line, right_line) {
                    (Some(l), Some(r)) if l.value == r.value => ChangeType::Equal,
                    (Some(_), Some(_)) => ChangeType::Replace,
                    (Some(_), None) => ChangeType::Delete,
                    (None, Some(_)) => ChangeType::Insert,
                    (None, None) => unreachable!(),
                };

                out.push(DiffLine {
                    old_line_num: left_line.and_then(|l| l.source_line_no),
                    new_line_num: right_line.and_then(|r| r.target_line_no),
                    old_content: left_line.map(|l| l.value.clone()),
                    new_content: right_line.map(|r| r.value.clone()),
                    change_type,
                });
            }
        }

        i = j;
    }
}

fn strip_git_prefix(path: &str) -> String {
    path.trim_start_matches("a/")
        .trim_start_matches("b/")
        .to_string()
}

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

/// Build inline segments for left and right side from a pair of strings.
fn inline_segments(old: &str, new: &str) -> (Vec<(String, bool)>, Vec<(String, bool)>) {
    let diff = TextDiff::from_chars(old, new);

    let mut left = Vec::new();
    let mut right = Vec::new();

    for change in diff.iter_all_changes() {
        let text = change.value().to_string();
        match change.tag() {
            ChangeTag::Equal => {
                left.push((text.clone(), false));
                right.push((text, false));
            }
            ChangeTag::Delete => {
                left.push((text, true));
            }
            ChangeTag::Insert => {
                right.push((text, true));
            }
        }
    }

    (left, right)
}

fn paint_inline_text(
    ui: &mut egui::Ui,
    segments: &[(String, bool)],
    base_color: egui::Color32,
    accent_color: egui::Color32,
) {
    let mut job = LayoutJob::default();

    for (text, highlight) in segments {
        let format = TextFormat {
            font_id: FontId::monospace(DIFF_FONT_SIZE),
            color: if *highlight { accent_color } else { base_color },
            ..Default::default()
        };
        job.append(text, 0.0, format);
    }

    ui.label(job);
}

pub fn render_diff_editor(ui: &mut egui::Ui, code: &str, _language: &str) {
    let theme = AppTheme::default();

    let files = match parse_diff_by_files(code) {
        Ok(files) => files,
        Err(err) => {
            ui.colored_label(
                theme.diff_removed_text,
                format!("Failed to parse diff: {err}"),
            );
            return;
        }
    };

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.heading(egui::RichText::new("📄 Diff").color(theme.text_primary));
            ui.label(
                egui::RichText::new(format!("({} files)", files.len()))
                    .color(theme.text_secondary)
                    .weak(),
            );
        });

        ui.add_space(4.0);

        if files.is_empty() {
            ui.label(
                egui::RichText::new("No diff detected")
                    .italics()
                    .color(theme.text_secondary),
            );
            return;
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for (idx, file) in files.iter().enumerate() {
                    let id = egui::Id::new("file_diff").with(idx);

                    let header_response = ui.horizontal(|ui| {
                        let is_open = ui.data(|d| d.get_temp::<bool>(id).unwrap_or(true));

                        let arrow = if is_open { "▼" } else { "▶" };
                        if ui
                            .button(egui::RichText::new(arrow).size(DIFF_FONT_SIZE))
                            .clicked()
                        {
                            ui.data_mut(|d| d.insert_temp(id, !is_open));
                        }

                        let display_path = if file.new_path != "/dev/null" {
                            &file.new_path
                        } else {
                            &file.old_path
                        };

                        ui.label(
                            egui::RichText::new(display_path)
                                .strong()
                                .color(theme.text_primary)
                                .size(DIFF_HEADER_FONT_SIZE),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if file.deletions > 0 {
                                ui.label(
                                    egui::RichText::new(format!("-{}", file.deletions))
                                        .color(theme.diff_removed_text)
                                        .size(DIFF_FONT_SIZE),
                                );
                            }
                            if file.additions > 0 {
                                ui.label(
                                    egui::RichText::new(format!("+{}", file.additions))
                                        .color(theme.diff_added_text)
                                        .size(DIFF_FONT_SIZE),
                                );
                            }
                        });

                        is_open
                    });

                    ui.separator();

                    if header_response.inner {
                        ui.columns(2, |columns| {
                            columns[0].vertical(|ui| {
                                ui.label(
                                    egui::RichText::new("Before")
                                        .strong()
                                        .color(theme.text_primary)
                                        .size(DIFF_FONT_SIZE),
                                );
                                ui.separator();

                                for line in &file.lines {
                                    render_line_left(ui, line, &theme);
                                }
                            });

                            columns[1].vertical(|ui| {
                                ui.label(
                                    egui::RichText::new("After")
                                        .strong()
                                        .color(theme.text_primary)
                                        .size(DIFF_FONT_SIZE),
                                );
                                ui.separator();

                                for line in &file.lines {
                                    render_line_right(ui, line, &theme);
                                }
                            });
                        });
                    }

                    ui.add_space(8.0);
                }
            });
    });
}

fn render_line_left(ui: &mut egui::Ui, line: &DiffLine, theme: &AppTheme) {
    // Color configuration. Insert uses transparent background.
    let (bg_color, text_color) = match line.change_type {
        ChangeType::Delete | ChangeType::Replace => {
            (theme.diff_removed_bg, theme.diff_removed_text)
        }
        ChangeType::Equal | ChangeType::Insert => (egui::Color32::TRANSPARENT, theme.text_primary),
    };

    ui.horizontal(|ui| {
        // Line number column
        if let Some(num) = line.old_line_num {
            ui.label(
                egui::RichText::new(format!("{:>4} ", num))
                    .color(theme.diff_line_num)
                    .monospace()
                    .size(DIFF_FONT_SIZE),
            );
        } else {
            ui.label(
                egui::RichText::new("     ")
                    .monospace()
                    .size(DIFF_FONT_SIZE),
            );
        }

        // Cell frame, even for insert only rows so height stays consistent
        let frame = egui::Frame::default()
            .fill(bg_color)
            .inner_margin(egui::Margin::symmetric(2, 0));

        frame.show(ui, |ui| match line.change_type {
            ChangeType::Insert => {
                // Placeholder so the row has the same height
                ui.label(
                    egui::RichText::new(" ")
                        .color(text_color)
                        .monospace()
                        .size(DIFF_FONT_SIZE),
                );
            }
            _ => {
                if let Some(content) = &line.old_content {
                    if line.change_type == ChangeType::Replace {
                        let (segments, _) = inline_segments(
                            content,
                            line.new_content.as_deref().unwrap_or_default(),
                        );
                        paint_inline_text(ui, &segments, theme.text_primary, text_color);
                    } else {
                        ui.label(
                            egui::RichText::new(content)
                                .color(text_color)
                                .monospace()
                                .size(DIFF_FONT_SIZE),
                        );
                    }
                } else {
                    // No content but keep the row height
                    ui.label(
                        egui::RichText::new(" ")
                            .color(text_color)
                            .monospace()
                            .size(DIFF_FONT_SIZE),
                    );
                }
            }
        });
    });
}

fn render_line_right(ui: &mut egui::Ui, line: &DiffLine, theme: &AppTheme) {
    // Color configuration. Delete uses transparent background on the empty side.
    let (bg_color, text_color) = match line.change_type {
        ChangeType::Insert | ChangeType::Replace => (theme.diff_added_bg, theme.diff_added_text),
        ChangeType::Equal | ChangeType::Delete => (egui::Color32::TRANSPARENT, theme.text_primary),
    };

    ui.horizontal(|ui| {
        // Line number column
        if let Some(num) = line.new_line_num {
            ui.label(
                egui::RichText::new(format!("{:>4} ", num))
                    .color(theme.diff_line_num)
                    .monospace()
                    .size(DIFF_FONT_SIZE),
            );
        } else {
            ui.label(
                egui::RichText::new("     ")
                    .monospace()
                    .size(DIFF_FONT_SIZE),
            );
        }

        // Cell frame, always present
        let frame = egui::Frame::default()
            .fill(bg_color)
            .inner_margin(egui::Margin::symmetric(2, 0));

        frame.show(ui, |ui| match line.change_type {
            ChangeType::Delete => {
                // Placeholder on the empty side
                ui.label(
                    egui::RichText::new(" ")
                        .color(text_color)
                        .monospace()
                        .size(DIFF_FONT_SIZE),
                );
            }
            _ => {
                if let Some(content) = &line.new_content {
                    if line.change_type == ChangeType::Replace {
                        let (_, segments) = inline_segments(
                            line.old_content.as_deref().unwrap_or_default(),
                            content,
                        );
                        paint_inline_text(ui, &segments, theme.text_primary, text_color);
                    } else {
                        ui.label(
                            egui::RichText::new(content)
                                .color(text_color)
                                .monospace()
                                .size(DIFF_FONT_SIZE),
                        );
                    }
                } else {
                    ui.label(
                        egui::RichText::new(" ")
                            .color(text_color)
                            .monospace()
                            .size(DIFF_FONT_SIZE),
                    );
                }
            }
        });
    });
}
