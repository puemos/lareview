use super::model::{ChangeType, DiffLine, FileDiff};
use similar::{ChangeTag, TextDiff};
use std::sync::Arc;
use unidiff::{Hunk, PatchSet, Result as UnidiffResult};

const MAX_INLINE_LEN: usize = 600;

fn should_do_inline(old: &str, new: &str) -> bool {
    old.len() <= MAX_INLINE_LEN && new.len() <= MAX_INLINE_LEN
}

fn strip_git_prefix(path: &str) -> String {
    path.trim_start_matches("a/")
        .trim_start_matches("b/")
        .to_string()
}

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

fn build_lines_for_hunk(hunk: &Hunk, out: &mut Vec<DiffLine>) {
    let lines = hunk.lines();
    let mut i = 0usize;

    while i < lines.len() {
        let line = &lines[i];

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

        if !removed.is_empty() && !added.is_empty() && removed.len() == added.len() {
            let mut has_similar = false;

            for (old_line, new_line) in removed.iter().zip(added.iter()) {
                let old_text = old_line.value.as_str();
                let new_text = new_line.value.as_str();

                let similarity = similar::TextDiff::from_chars(old_text, new_text).ratio();

                if similarity > 0.3 {
                    has_similar = true;
                    break;
                }
            }

            if has_similar {
                for (old_line, new_line) in removed.iter().zip(added.iter()) {
                    let old_text = old_line.value.as_str();
                    let new_text = new_line.value.as_str();

                    let similarity = similar::TextDiff::from_chars(old_text, new_text).ratio();

                    if similarity > 0.3 && should_do_inline(old_text, new_text) {
                        out.push(DiffLine {
                            old_line_num: old_line.source_line_no,
                            new_line_num: None,
                            content: Arc::from(old_text),
                            change_type: ChangeType::Delete,
                            inline_segments: Some(inline_segments(old_text, new_text)),
                        });

                        out.push(DiffLine {
                            old_line_num: None,
                            new_line_num: new_line.target_line_no,
                            content: Arc::from(new_text),
                            change_type: ChangeType::Insert,
                            inline_segments: Some(inline_segments(new_text, old_text)),
                        });
                    } else {
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

pub(super) fn parse_diff_by_files(diff_text: &str) -> UnidiffResult<Vec<FileDiff>> {
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
