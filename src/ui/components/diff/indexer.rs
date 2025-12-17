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
    let mut old_start = 1;
    let mut old_len = 0;
    let mut new_start = 1;
    let mut new_len = 0;

    // Extract the hunk info part from the header
    let header_parts: Vec<&str> = hunk_header.split("@@").collect();
    if header_parts.len() > 1 {
        let hunk_info = header_parts[0].trim_start_matches('@');
        let info_parts: Vec<&str> = hunk_info.split_whitespace().collect();

        for part in info_parts {
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
    }

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

    for (idx, line_idx) in body_range.clone().enumerate() {
        // Update line numbers based on line type
        if (idx + start_idx + 1) < lines.len() {
            let line = lines[idx + start_idx + 1];
            match get_change_type_from_line(line) {
                ChangeType::Insert => {
                    if idx % CHECKPOINT_INTERVAL as usize == 0 {
                        checkpoints.push(Checkpoint {
                            at_line: line_idx,
                            old_no: current_old,
                            new_no: current_new,
                        });
                    }
                    current_new += 1;
                }
                ChangeType::Delete => {
                    if idx % CHECKPOINT_INTERVAL as usize == 0 {
                        checkpoints.push(Checkpoint {
                            at_line: line_idx,
                            old_no: current_old,
                            new_no: current_new,
                        });
                    }
                    current_old += 1;
                }
                ChangeType::Equal => {
                    if idx % CHECKPOINT_INTERVAL as usize == 0 {
                        checkpoints.push(Checkpoint {
                            at_line: line_idx,
                            old_no: current_old,
                            new_no: current_new,
                        });
                    }
                    current_old += 1;
                    current_new += 1;
                }
            }
        }
    }

    // Add final checkpoint if there isn't one at the end
    if checkpoints.is_empty() || checkpoints.last().unwrap().at_line < body_range.end - 1 {
        checkpoints.push(Checkpoint {
            at_line: body_range.end - 1,
            old_no: current_old,
            new_no: current_new,
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
