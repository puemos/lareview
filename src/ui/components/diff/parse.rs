// This file is no longer used in the new architecture, so we can remove the old types
use super::model::ChangeType;
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

// This function is kept for compatibility, but the new architecture doesn't use this approach
#[allow(dead_code)]
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

// This function is kept for compatibility with the old code that might still reference it
#[allow(dead_code)]
fn parse_diff_by_files(diff_text: &str) -> UnidiffResult<Vec<FileDiff>> {
    let trimmed = diff_text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let mut patch = PatchSet::new();
    patch.parse(trimmed)?;

    let mut files_out: Vec<FileDiff> = Vec::new();
    let mut file_indices: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for file in patch.files() {
        let mut lines = Vec::new();

        for hunk in file.hunks() {
            build_lines_for_hunk(hunk, &mut lines);
        }

        let old_path = strip_git_prefix(&file.source_file);
        let new_path = strip_git_prefix(&file.target_file);

        // Determine the key for aggregation.
        // We use the same logic as the UI to determine the "primary" path.
        let key = if new_path != "/dev/null" {
            new_path.clone()
        } else {
            old_path.clone()
        };

        if let Some(&idx) = file_indices.get(&key) {
            // Aggregate into existing entry
            files_out[idx].lines.extend(lines);
            files_out[idx].additions += file.added();
            files_out[idx].deletions += file.removed();
        } else {
            // New entry
            let idx = files_out.len();
            file_indices.insert(key, idx);

            files_out.push(FileDiff {
                old_path,
                new_path,
                lines,
                additions: file.added(),
                deletions: file.removed(),
            });
        }
    }

    Ok(files_out)
}

// Define the old types here so that this file can compile, but they're no longer used in the new architecture
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct DiffLine {
    old_line_num: Option<usize>,
    new_line_num: Option<usize>,
    content: Arc<str>,
    change_type: ChangeType,
    inline_segments: Option<Vec<(String, bool)>>, // (text, highlight)
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct FileDiff {
    old_path: String,
    new_path: String,
    lines: Vec<DiffLine>,
    additions: usize,
    deletions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_diff_aggregates_duplicates() {
        // A diff where the same file appears twice (simulating multiple hunks/sections for the same file)
        let diff = r#"
diff --git a/file.rs b/file.rs
index 1..2 100644
--- a/file.rs
+++ b/file.rs
@@ -1,1 +1,2 @@
 line1
+line2
diff --git a/file.rs b/file.rs
index 2..3 100644
--- a/file.rs
+++ b/file.rs
@@ -10,1 +11,2 @@
 line10
+line11
"#;
        let files = parse_diff_by_files(diff).expect("Failed to parse");

        // Should be aggregated into 1 file
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].new_path, "file.rs");
        // 1 hunk from first part, 1 hunk from second part -> total lines depends on implementation
        // But mainly we check file count.
        assert_eq!(files[0].additions, 2);
    }
}
