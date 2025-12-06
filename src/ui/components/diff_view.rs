//! Diff view component
//!
//! Handles parsing and rendering of unified diffs into a Split View representation.
//! Designed to work with virtualized lists for performance.

use gpui::{div, prelude::*, px};
use similar::{ChangeTag, TextDiff};

use crate::ui::theme::theme;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffRowType {
    Header,
    Context,
    Addition,
    Deletion,
    Modification,
}

#[derive(Debug, Clone)]
pub struct DiffSegment {
    pub text: String,
    pub highlight: bool,
}

impl DiffSegment {
    fn new(text: String, highlight: bool) -> Self {
        Self { text, highlight }
    }
}

#[derive(Debug, Clone)]
pub struct DiffRow {
    pub left_no: Option<u32>,
    pub left_content: Option<Vec<DiffSegment>>,
    pub right_no: Option<u32>,
    pub right_content: Option<Vec<DiffSegment>>,
    pub kind: DiffRowType,
}

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub file_path: String,
    pub patch: String,
}

#[derive(Debug, Clone)]
pub enum DiffItem {
    FileHeader(String),
    HunkHeader(String),
    Row(DiffRow),
}

pub fn parse_diff(input: &str) -> Vec<FileDiff> {
    let mut diffs = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut current_file: Option<String> = None;
    let mut current_patch = Vec::new();

    for line in lines {
        if line.starts_with("diff --git") {
            if let Some(path) = current_file.take() {
                if !current_patch.is_empty() {
                    diffs.push(FileDiff {
                        file_path: path,
                        patch: current_patch.join("\n"),
                    });
                    current_patch.clear();
                }
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let b_path = parts[3];
                current_file = Some(b_path.strip_prefix("b/").unwrap_or(b_path).to_string());
            } else {
                current_file = Some("unknown".to_string());
            }
        }

        if current_file.is_some() {
            current_patch.push(line);
        }
    }

    if let Some(path) = current_file {
        if !current_patch.is_empty() {
            diffs.push(FileDiff {
                file_path: path,
                patch: current_patch.join("\n"),
            });
        }
    }

    if diffs.is_empty() && !input.trim().is_empty() {
        diffs.push(FileDiff {
            file_path: "pasted_diff".to_string(),
            patch: input.to_string(),
        });
    }

    diffs
}

pub fn process_diff_for_list(file_path: &str, patch: &str) -> Vec<DiffItem> {
    let mut items = Vec::new();
    items.push(DiffItem::FileHeader(file_path.to_string()));

    let mut old_no: u32 = 0;
    let mut new_no: u32 = 0;
    let mut pending_deletion: Option<(u32, String)> = None;

    for line in patch.lines() {
        if line.starts_with("@@") {
            if let Some((no, text)) = pending_deletion.take() {
                items.push(DiffItem::Row(DiffRow {
                    left_no: Some(no),
                    left_content: Some(vec![DiffSegment::new(text, false)]),
                    right_no: None,
                    right_content: None,
                    kind: DiffRowType::Deletion,
                }));
            }

            let mut parsed_old = None;
            let mut parsed_new = None;
            if let Some((old_part, new_rest)) = line.split_once(" +") {
                parsed_old = parse_hunk_range(old_part.trim_start_matches("@@ -"));
                parsed_new = parse_hunk_range(new_rest.split_whitespace().next().unwrap_or(""));
            }
            if let Some((start, _)) = parsed_old {
                old_no = start;
            }
            if let Some((start, _)) = parsed_new {
                new_no = start;
            }

            items.push(DiffItem::HunkHeader(line.to_string()));
            continue;
        }

        let char_code = line.chars().next();
        let content = line.chars().skip(1).collect::<String>();

        match char_code {
            Some('-') => {
                if let Some((no, text)) = pending_deletion.take() {
                    items.push(DiffItem::Row(DiffRow {
                        left_no: Some(no),
                        left_content: Some(vec![DiffSegment::new(text, false)]),
                        right_no: None,
                        right_content: None,
                        kind: DiffRowType::Deletion,
                    }));
                }
                pending_deletion = Some((old_no, content));
                old_no += 1;
            }
            Some('+') => {
                if let Some((d_no, d_text)) = pending_deletion.take() {
                    let (left_segs, right_segs) = compute_word_diff(&d_text, &content);
                    items.push(DiffItem::Row(DiffRow {
                        left_no: Some(d_no),
                        left_content: Some(left_segs),
                        right_no: Some(new_no),
                        right_content: Some(right_segs),
                        kind: DiffRowType::Modification,
                    }));
                } else {
                    items.push(DiffItem::Row(DiffRow {
                        left_no: None,
                        left_content: None,
                        right_no: Some(new_no),
                        right_content: Some(vec![DiffSegment::new(content, false)]),
                        kind: DiffRowType::Addition,
                    }));
                }
                new_no += 1;
            }
            Some(' ') => {
                if let Some((no, text)) = pending_deletion.take() {
                    items.push(DiffItem::Row(DiffRow {
                        left_no: Some(no),
                        left_content: Some(vec![DiffSegment::new(text, false)]),
                        right_no: None,
                        right_content: None,
                        kind: DiffRowType::Deletion,
                    }));
                }
                items.push(DiffItem::Row(DiffRow {
                    left_no: Some(old_no),
                    left_content: Some(vec![DiffSegment::new(content.clone(), false)]),
                    right_no: Some(new_no),
                    right_content: Some(vec![DiffSegment::new(content, false)]),
                    kind: DiffRowType::Context,
                }));
                old_no += 1;
                new_no += 1;
            }
            _ => {}
        }
    }

    if let Some((no, text)) = pending_deletion.take() {
        items.push(DiffItem::Row(DiffRow {
            left_no: Some(no),
            left_content: Some(vec![DiffSegment::new(text, false)]),
            right_no: None,
            right_content: None,
            kind: DiffRowType::Deletion,
        }));
    }

    items
}

fn compute_word_diff(old: &str, new: &str) -> (Vec<DiffSegment>, Vec<DiffSegment>) {
    let diff = TextDiff::from_words(old, new);
    let mut left_segs = Vec::new();
    let mut right_segs = Vec::new();

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Delete => {
                left_segs.push(DiffSegment::new(change.value().to_string(), true));
            }
            ChangeTag::Insert => {
                right_segs.push(DiffSegment::new(change.value().to_string(), true));
            }
            ChangeTag::Equal => {
                let text = change.value().to_string();
                left_segs.push(DiffSegment::new(text.clone(), false));
                right_segs.push(DiffSegment::new(text, false));
            }
        }
    }
    (left_segs, right_segs)
}

fn parse_hunk_range(range: &str) -> Option<(u32, u32)> {
    let mut parts = range.split(',');
    let start = parts.next()?.parse().ok()?;
    let len = parts.next()?.parse().ok()?;
    Some((start, len))
}

pub fn render_diff_view(file_path: &str, patch: &str) -> impl IntoElement {
    let items = process_diff_for_list(file_path, patch);
    div()
        .flex()
        .flex_col()
        .children(
            items
                .into_iter()
                .enumerate()
                .map(|(ix, item)| render_diff_item(item, ix)),
        )
}

pub fn render_diff_item(item: DiffItem, ix: usize) -> impl IntoElement {
    let colors = theme().colors;
    let spacing = theme().spacing;

    match item {
        DiffItem::FileHeader(path) => div()
            .bg(colors.surface_alt)
            .border_y_1()
            .border_color(colors.border)
            .px(px(spacing.space_4))
            .py(px(spacing.space_2))
            .mt(if ix > 0 {
                px(spacing.space_6)
            } else {
                px(0.0)
            })
            .font_weight(gpui::FontWeight::BOLD)
            .text_sm()
            .text_color(colors.text)
            .child(path.to_string())
            .into_any_element(),
        DiffItem::HunkHeader(header) => div()
            .bg(colors.surface_alt.opacity(0.5))
            .px(px(spacing.space_4))
            .py(px(spacing.space_1))
            .border_b_1()
            .border_color(colors.border.opacity(0.5))
            .font_family("JetBrains Mono")
            .text_xs()
            .text_color(colors.text_muted.opacity(0.8))
            .child(header.to_string())
            .into_any_element(),
        DiffItem::Row(row) => div()
            .flex()
            .w_full()
            .font_family("JetBrains Mono")
            .text_sm()
            .border_b_1()
            .border_color(colors.border.opacity(0.1))
            .child(render_half_row(
                row.left_no,
                row.left_content.as_deref(),
                &row.kind,
                true,
            ))
            .child(div().w(px(1.0)).bg(colors.border.opacity(0.2)))
            .child(render_half_row(
                row.right_no,
                row.right_content.as_deref(),
                &row.kind,
                false,
            ))
            .into_any_element(),
    }
}

fn render_half_row(
    line_no: Option<u32>,
    content: Option<&[DiffSegment]>,
    kind: &DiffRowType,
    is_left: bool,
) -> impl IntoElement {
    let colors = theme().colors;
    let spacing = theme().spacing;

    let (bg, gutter_bg, text_color, highlight_bg) = match kind {
        DiffRowType::Addition => {
            if is_left {
                (
                    gpui::Hsla::default(),
                    gpui::Hsla::default(),
                    colors.text_muted.opacity(0.0),
                    gpui::Hsla::default(),
                )
            } else {
                (
                    colors.success.opacity(0.1),
                    colors.success.opacity(0.2),
                    colors.text,
                    colors.success.opacity(0.3),
                )
            }
        }
        DiffRowType::Deletion => {
            if is_left {
                (
                    colors.danger.opacity(0.1),
                    colors.danger.opacity(0.2),
                    colors.text,
                    colors.danger.opacity(0.3),
                )
            } else {
                (
                    gpui::Hsla::default(),
                    gpui::Hsla::default(),
                    colors.text_muted.opacity(0.0),
                    gpui::Hsla::default(),
                )
            }
        }
        DiffRowType::Modification => {
            if is_left {
                (
                    colors.danger.opacity(0.1),
                    colors.danger.opacity(0.2),
                    colors.text,
                    colors.danger.opacity(0.25),
                )
            } else {
                (
                    colors.success.opacity(0.1),
                    colors.success.opacity(0.2),
                    colors.text,
                    colors.success.opacity(0.25),
                )
            }
        }
        _ => (
            colors.bg,
            colors.surface_alt.opacity(0.5),
            colors.text,
            gpui::Hsla::default(),
        ),
    };

    let is_empty = content.is_none();
    let final_bg = if is_empty {
        colors.surface_alt.opacity(0.15)
    } else {
        bg
    };

    div()
        .flex_1()
        .flex()
        .bg(final_bg)
        .min_h(px(20.0))
        .child(
            div()
                .w(px(40.0))
                .bg(if is_empty {
                    gpui::Hsla::default()
                } else {
                    gutter_bg
                })
                .text_xs()
                .text_color(colors.text_muted)
                .flex()
                .justify_end()
                .items_center()
                .pr(px(spacing.space_2))
                .child(line_no.map(|n| n.to_string()).unwrap_or_default()),
        )
        .child(
            div()
                .flex_1()
                .pl(px(spacing.space_2))
                .items_center()
                .flex()
                .flex_wrap()
                .text_color(text_color)
                .whitespace_nowrap()
                .children(content.unwrap_or(&[]).iter().map(|seg| {
                    if seg.highlight {
                        div().bg(highlight_bg).child(seg.text.clone())
                    } else {
                        div().child(seg.text.clone())
                    }
                })),
        )
}
