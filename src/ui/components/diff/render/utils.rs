use super::super::doc::DiffDoc;
use super::super::model::ChangeType;
use eframe::egui::{self, FontId, TextFormat, text::LayoutJob};

pub const DIFF_FONT_SIZE: f32 = 12.0;
pub const HEADER_FONT_SIZE: f32 = 14.0;

pub fn should_render_line(line: &str) -> bool {
    !line.starts_with("diff --git ")
        && !line.starts_with("--- ")
        && !line.starts_with("+++ ")
        && !line.starts_with("@@ ")
        && !line.starts_with("index ")
}

pub fn calculate_total_rows(doc: &DiffDoc, collapsed: &[bool]) -> usize {
    let mut total = 0;
    for (file_idx, file) in doc.files.iter().enumerate() {
        total += 1;
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

pub fn get_row_type(
    doc: &DiffDoc,
    collapsed: &[bool],
    row_idx: usize,
) -> Option<super::types::RowType> {
    let mut current_row = 0;
    for (file_idx, file) in doc.files.iter().enumerate() {
        if current_row == row_idx {
            return Some(super::types::RowType::FileHeader { file_idx });
        }
        current_row += 1;

        if file_idx < collapsed.len() && !collapsed[file_idx] {
            for line_idx in file.line_range.clone() {
                let line_str = doc.line_str(line_idx);
                if should_render_line(line_str) {
                    if current_row == row_idx {
                        return Some(super::types::RowType::DiffLine { file_idx, line_idx });
                    }
                    current_row += 1;
                }
            }
        }
    }
    None
}

pub fn extract_line_numbers(
    doc: &DiffDoc,
    file_idx: usize,
    line_idx: u32,
) -> (Option<usize>, Option<usize>) {
    let (old_line, new_line) =
        super::super::indexer::calculate_line_numbers(doc, file_idx, line_idx);
    (old_line.map(|x| x as usize), new_line.map(|x| x as usize))
}

pub fn strip_git_prefix(path: &str) -> String {
    path.trim_start_matches("a/")
        .trim_start_matches("b/")
        .to_string()
}

pub fn middle_truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    let keep = (max_len - 3) / 2;
    let start = &s[..keep];
    let end = &s[s.len() - keep..];
    format!("{}...{}", start, end)
}

pub fn compute_inline_diff_if_appropriate(
    line_str: &str,
    _change_type: &ChangeType,
) -> Vec<(String, bool)> {
    match _change_type {
        ChangeType::Equal => vec![(line_str.to_string(), false)],
        ChangeType::Delete => vec![(line_str.trim_start_matches('-').to_string(), true)],
        ChangeType::Insert => vec![(line_str.trim_start_matches('+').to_string(), true)],
    }
}

pub fn paint_inline_text_job(
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

pub fn strip_diff_prefix(content: &str, change_type: &ChangeType) -> String {
    match change_type {
        ChangeType::Equal => content.to_string(),
        ChangeType::Delete => content.trim_start_matches('-').to_string(),
        ChangeType::Insert => content.trim_start_matches('+').to_string(),
    }
}
