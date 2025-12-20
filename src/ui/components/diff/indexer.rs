use std::sync::Arc;

use super::{
    doc::{Checkpoint, DiffDoc, FileIndex, HunkIndex},
    model::ChangeType,
};

const CHECKPOINT_INTERVAL: u32 = 256; // Store checkpoint every N lines

pub fn index_diff(diff_text: &str) -> DiffDoc {
    let text: Arc<str> = Arc::from(diff_text);
    let mut doc = DiffDoc::new(text);

    // Parse files and hunks from the diff text
    let lines: Vec<&str> = diff_text.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        if lines[i].starts_with("diff --git ") {
            if let Some(_file_idx) = parse_file(&lines, i, &mut doc) {
                // Move to the line after the current file
                i = find_next_file_start(&lines, i + 1);
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    doc
}

fn find_next_file_start(lines: &[&str], start_idx: usize) -> usize {
    for (i, line) in lines.iter().enumerate().skip(start_idx) {
        if line.starts_with("diff --git ") {
            return i;
        }
    }
    lines.len()
}

fn parse_file(lines: &[&str], start_idx: usize, doc: &mut DiffDoc) -> Option<usize> {
    if start_idx >= lines.len() || !lines[start_idx].starts_with("diff --git ") {
        return None;
    }

    // Parse the file header
    let mut current_idx = start_idx + 1;
    let mut old_path = String::new();
    let mut new_path = String::new();

    // Look for --- and +++ lines
    while current_idx < lines.len() {
        if lines[current_idx].starts_with("--- ") {
            old_path = lines[current_idx][4..].trim_start().to_string();
        } else if lines[current_idx].starts_with("+++ ") {
            new_path = lines[current_idx][4..].trim_start().to_string();
        } else if lines[current_idx].starts_with("@@ ") {
            // Found first hunk, stop looking for file headers
            break;
        }
        current_idx += 1;
    }

    // Find the end of this file section
    let file_end_line_idx = find_file_end(lines, current_idx);

    // Parse hunks in this file section
    let mut hunks = Vec::new();
    let mut current_line_idx = start_idx as u32;
    let file_start_line_idx = current_line_idx;
    let mut file_additions = 0u32;
    let mut file_deletions = 0u32;

    // Process lines from file header to end of file
    let mut idx = start_idx;
    while idx < file_end_line_idx && idx < lines.len() {
        if lines[idx].starts_with("@@ ") {
            if let Some((hunk, hunk_lines_parsed)) = parse_hunk(lines, idx, current_line_idx) {
                file_additions += hunk.additions;
                file_deletions += hunk.deletions;
                hunks.push(hunk);
                current_line_idx += hunk_lines_parsed as u32;
                idx += hunk_lines_parsed;
            } else {
                current_line_idx += 1;
                idx += 1;
            }
        } else {
            current_line_idx += 1;
            idx += 1;
        }
    }

    let file = FileIndex {
        old_path,
        new_path,
        line_range: file_start_line_idx..current_line_idx,
        additions: file_additions,
        deletions: file_deletions,
        hunks,
    };

    let file_idx = doc.files.len();
    doc.files.push(file);
    Some(file_idx)
}

fn find_file_end(lines: &[&str], hunk_start_idx: usize) -> usize {
    for (i, line) in lines.iter().enumerate().skip(hunk_start_idx) {
        if line.starts_with("diff --git ") {
            return i;
        }
    }
    lines.len()
}

fn parse_hunk(lines: &[&str], start_idx: usize, start_line_idx: u32) -> Option<(HunkIndex, usize)> {
    if start_idx >= lines.len() || !lines[start_idx].starts_with("@@ ") {
        return None;
    }

    let hunk_header = lines[start_idx];
    let header_line = start_line_idx;

    // Parse old and new line info from header: @@ -old_start,old_len +new_start,new_len @@
    let (old_start, old_len, new_start, new_len) = parse_hunk_header(hunk_header)
        // Fall back to defaults if parsing fails so rendering still works
        .unwrap_or((1, 0, 1, 0));

    // Count the actual lines in the hunk body
    let body_start_line_idx = start_line_idx + 1;
    let mut body_line_count = 0;
    let mut additions = 0u32;
    let mut deletions = 0u32;

    let mut current_idx = start_idx + 1;
    while current_idx < lines.len()
        && !lines[current_idx].starts_with("@@ ")
        && !lines[current_idx].starts_with("diff --git ")
    {
        let line = lines[current_idx];
        if line.starts_with('+') {
            additions += 1;
        } else if line.starts_with('-') {
            deletions += 1;
        }
        body_line_count += 1;
        current_idx += 1;
    }

    let body_range = body_start_line_idx..(body_start_line_idx + body_line_count as u32);

    // Create checkpoints every N lines in the hunk body
    let mut checkpoints = Vec::new();
    let mut current_old = old_start;
    let mut current_new = new_start;
    let mut last_line_numbers: Option<(u32, u32, u32)> = None;

    for (idx, line_idx) in body_range.clone().enumerate() {
        // Update line numbers based on line type
        if (idx + start_idx + 1) < lines.len() {
            let line = lines[idx + start_idx + 1];
            let change_type = get_change_type_from_line(line);
            let is_checkpoint = idx % CHECKPOINT_INTERVAL as usize == 0;

            if is_checkpoint {
                checkpoints.push(Checkpoint {
                    at_line: line_idx,
                    old_no: current_old,
                    new_no: current_new,
                });
            }

            // Record the numbers for this line before advancing
            last_line_numbers = Some((line_idx, current_old, current_new));

            match change_type {
                ChangeType::Insert => {
                    current_new += 1;
                }
                ChangeType::Delete => {
                    current_old += 1;
                }
                ChangeType::Equal => {
                    current_old += 1;
                    current_new += 1;
                }
            }
        }
    }

    // Add final checkpoint if there isn't one at the end
    if let Some((last_idx, last_old, last_new)) = last_line_numbers
        && checkpoints
            .last()
            .map(|cp| cp.at_line != last_idx)
            .unwrap_or(true)
    {
        checkpoints.push(Checkpoint {
            at_line: last_idx,
            old_no: last_old,
            new_no: last_new,
        });
    }

    // Use old_len, new_len, header_line, old_start, new_start to avoid unused variable warnings
    // These values from the header might differ from actual line counts
    let _ = (old_len, new_len, header_line, old_start, new_start);

    let hunk = HunkIndex {
        body_range,
        checkpoints,
        additions,
        deletions,
    };

    Some((hunk, body_line_count + 1)) // +1 for the header line
}

fn parse_hunk_header(header: &str) -> Option<(u32, u32, u32, u32)> {
    if !header.starts_with("@@") {
        return None;
    }

    // Trim the leading "@@" and capture the segment before the next "@@"
    let rest = header.trim_start_matches('@');
    let (meta, _) = rest.split_once("@@")?;
    let mut old_start = 1;
    let mut old_len = 0;
    let mut new_start = 1;
    let mut new_len = 0;

    for part in meta.split_whitespace() {
        if let Some(stripped) = part.strip_prefix('-') {
            let nums: Vec<&str> = stripped.split(',').collect();
            if let Some(start_str) = nums.first() {
                old_start = start_str.parse().unwrap_or(1);
            }
            if let Some(len_str) = nums.get(1) {
                old_len = len_str.parse().unwrap_or(0);
            }
        } else if let Some(stripped) = part.strip_prefix('+') {
            let nums: Vec<&str> = stripped.split(',').collect();
            if let Some(start_str) = nums.first() {
                new_start = start_str.parse().unwrap_or(1);
            }
            if let Some(len_str) = nums.get(1) {
                new_len = len_str.parse().unwrap_or(0);
            }
        }
    }

    Some((old_start, old_len, new_start, new_len))
}

// Determine change type for a line
pub fn get_change_type_from_line(line: &str) -> ChangeType {
    // File and hunk headers are treated as context (Equal)
    if line.starts_with("diff --git ")
        || line.starts_with("--- ")
        || line.starts_with("+++ ")
        || line.starts_with("@@ ")
    {
        ChangeType::Equal
    } else if line.starts_with('+') {
        ChangeType::Insert
    } else if line.starts_with('-') {
        ChangeType::Delete
    } else {
        ChangeType::Equal
    }
}

// Efficiently calculate old and new line numbers for a given diff line
pub fn calculate_line_numbers(
    doc: &DiffDoc,
    file_idx: usize,
    line_idx: u32,
) -> (Option<u32>, Option<u32>) {
    if file_idx >= doc.files.len() {
        return (None, None);
    }

    let file = &doc.files[file_idx];
    if !file.line_range.contains(&line_idx) {
        return (None, None);
    }

    // Find which hunk contains this line
    for hunk in &file.hunks {
        if hunk.body_range.contains(&line_idx) {
            // Find the nearest checkpoint before this line
            let mut checkpoint = None;
            for cp in hunk.checkpoints.iter().rev() {
                if cp.at_line <= line_idx {
                    checkpoint = Some(cp);
                    break;
                }
            }

            if let Some(cp) = checkpoint {
                // Calculate line numbers from the checkpoint
                let mut old_line = cp.old_no;
                let mut new_line = cp.new_no;

                // Iterate from the checkpoint to the target line
                for current_idx in cp.at_line..line_idx {
                    if current_idx < doc.line_count() as u32 {
                        let line_text = doc.line_str(current_idx);
                        match get_change_type_from_line(line_text) {
                            ChangeType::Insert => {
                                new_line += 1;
                            }
                            ChangeType::Delete => {
                                old_line += 1;
                            }
                            ChangeType::Equal => {
                                old_line += 1;
                                new_line += 1;
                            }
                        }
                    }
                }

                // Now determine the numbers for the target line
                let line_text = doc.line_str(line_idx);
                match get_change_type_from_line(line_text) {
                    ChangeType::Insert => {
                        return (None, Some(new_line));
                    }
                    ChangeType::Delete => {
                        return (Some(old_line), None);
                    }
                    ChangeType::Equal => {
                        return (Some(old_line), Some(new_line));
                    }
                }
            }
        }
    }

    // If we couldn't find the line in any hunk (e.g., it's a file header line)
    (None, None)
}

#[cfg(test)]
mod tests {
    use super::{calculate_line_numbers, index_diff};

    #[test]
    fn calculates_line_numbers_for_mid_file_hunk() {
        let diff = r#"diff --git a/file.txt b/file.txt
--- a/file.txt
+++ b/file.txt
@@ -10,2 +10,3 @@
 line10
+line10.5
 line11
"#;

        let doc = index_diff(diff);
        assert_eq!(doc.files.len(), 1);
        let file = &doc.files[0];
        assert_eq!(file.hunks.len(), 1);
        let hunk = &file.hunks[0];

        let first_line_idx = hunk.body_range.start;
        let insert_line_idx = hunk.body_range.start + 1;
        let last_line_idx = hunk.body_range.end - 1;

        let (old_first, new_first) = calculate_line_numbers(&doc, 0, first_line_idx);
        assert_eq!((old_first, new_first), (Some(10), Some(10)));

        let (old_insert, new_insert) = calculate_line_numbers(&doc, 0, insert_line_idx);
        assert_eq!((old_insert, new_insert), (None, Some(11)));

        let (old_last, new_last) = calculate_line_numbers(&doc, 0, last_line_idx);
        assert_eq!((old_last, new_last), (Some(11), Some(12)));
    }

    #[test]
    fn does_not_shift_last_line_numbers() {
        let diff = r#"diff --git a/file.txt b/file.txt
--- a/file.txt
+++ b/file.txt
@@ -20,3 +20,3 @@
 line20
-line21
+line21-updated
 line22
"#;

        let doc = index_diff(diff);
        assert_eq!(doc.files.len(), 1);
        let file = &doc.files[0];
        assert_eq!(file.hunks.len(), 1);
        let hunk = &file.hunks[0];

        let last_line_idx = hunk.body_range.end - 1;
        let (old_last, new_last) = calculate_line_numbers(&doc, 0, last_line_idx);
        assert_eq!((old_last, new_last), (Some(22), Some(22)));
    }
}
