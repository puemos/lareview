//! A module for indexing and querying unified diffs.
//!
//! This module provides a `DiffIndex` that can be created from a unified diff string.
//! The index allows for efficient querying of diff statistics and reconstruction of
//! partial diffs based on `DiffRef` pointers.

use crate::domain::{DiffRef, FeedbackSide, HunkRef};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use unidiff::{Hunk, PatchSet};

// serde_json is used for error serialization
use serde_json;

use std::fmt;

pub type LineRange = (u32, u32);

/// Errors that can occur when working with DiffIndex
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffIndexError {
    /// File not found in the diff index
    FileNotFound { file: String },
    /// Hunk not found in a file
    HunkNotFound {
        file: String,
        old_start: u32,
        new_start: u32,
    },
    /// Invalid hunk ID format
    InvalidHunkId { file: String, hunk_id: String },
    /// Parse error when processing the diff
    Parse { message: String },
    /// Nearest hunk information when a hunk is not found
    NearestHunk {
        file: String,
        old_start: u32,
        new_start: u32,
        nearest: Option<(u32, u32)>,
    },
}

impl fmt::Display for DiffIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiffIndexError::FileNotFound { file } => {
                write!(f, "File not found in diff index: {}", file)
            }
            DiffIndexError::HunkNotFound {
                file,
                old_start,
                new_start,
            } => {
                write!(
                    f,
                    "Hunk not found in file {}: old_start={}, new_start={}",
                    file, old_start, new_start
                )
            }
            DiffIndexError::InvalidHunkId { file, hunk_id } => {
                write!(f, "Invalid hunk ID in file {}: {}", file, hunk_id)
            }
            DiffIndexError::Parse { message } => {
                write!(f, "Parse error: {}", message)
            }
            DiffIndexError::NearestHunk {
                file,
                old_start,
                new_start,
                nearest,
            } => {
                write!(
                    f,
                    "Hunk not found at ({}, {}) in file {}, nearest hunk: {:?}",
                    old_start, new_start, file, nearest
                )
            }
        }
    }
}

impl std::error::Error for DiffIndexError {}

#[derive(Debug, Clone)]
pub struct LineMatch {
    pub line_number: u32,
    pub line_content: String,
    pub position_in_hunk: usize,
    pub is_addition: bool,
    pub is_deletion: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineLocation {
    pub position_in_hunk: usize,
    pub old_line_number: Option<u32>,
    pub new_line_number: Option<u32>,
    pub is_addition: bool,
    pub is_deletion: bool,
}

#[derive(Debug, Clone)]
pub struct HunkLineInfo {
    pub position_in_hunk: usize,
    pub content: String,
    pub is_addition: bool,
    pub is_deletion: bool,
    pub old_line_number: Option<u32>,
    pub new_line_number: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct HunkMatch {
    pub hunk_id: String,
    pub file_path: String,
    pub hunk_ref: HunkRef,
    pub lines: Vec<LineMatch>,
}

impl DiffIndexError {
    /// Convert the error to a JSON string representation
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| format!("{:?}", self))
    }

    /// Get the nearest hunk coordinates if this error contains them
    pub fn nearest(&self) -> Option<(u32, u32)> {
        match self {
            DiffIndexError::NearestHunk { nearest, .. } => *nearest,
            _ => None,
        }
    }
}

/// A hunk with its coordinates and potentially an ID
#[derive(Debug, Clone)]
pub struct IndexedHunk {
    pub hunk: Hunk,
    /// The coordinates of the hunk (old_start, new_start)
    pub coords: (u32, u32),
}

/// An index for a single file within a larger diff.
#[derive(Debug, Clone)]
pub struct FileIndex {
    /// A map from (old_start, new_start) to the indexed hunk.
    pub hunks: HashMap<(u32, u32), IndexedHunk>,
    /// A list of all available hunks in order.
    pub all_hunks: Vec<IndexedHunk>,
}

/// An index for a unified diff, allowing for efficient queries.
#[derive(Debug, Clone)]
pub struct DiffIndex {
    pub files: HashMap<String, FileIndex>,
}

impl DiffIndex {
    /// Creates a new `DiffIndex` from a unified diff string.
    pub fn new(diff_text: &str) -> Result<Self> {
        let trimmed = diff_text.trim();
        if trimmed.is_empty() {
            return Ok(Self {
                files: HashMap::new(),
            });
        }

        let mut patch_set = PatchSet::new();
        patch_set.parse(trimmed)?;

        let mut files = HashMap::new();

        for file in patch_set.files() {
            let mut file_path = file
                .target_file
                .strip_prefix("b/")
                .unwrap_or(&file.target_file);
            if file_path == "dev/null" || file_path == "/dev/null" {
                file_path = file
                    .source_file
                    .strip_prefix("a/")
                    .unwrap_or(&file.source_file);
            }

            let mut hunks = HashMap::new();
            let mut all_hunks = Vec::new();
            for hunk in file.hunks() {
                let coords = (hunk.source_start as u32, hunk.target_start as u32);
                let indexed_hunk = IndexedHunk {
                    hunk: hunk.clone(),
                    coords,
                };
                hunks.insert(coords, indexed_hunk.clone());
                all_hunks.push(indexed_hunk);
            }
            files.insert(file_path.to_string(), FileIndex { hunks, all_hunks });
        }

        Ok(DiffIndex { files })
    }

    /// Finds the hunk containing a specific line number.
    pub fn find_hunk_at_line(
        &self,
        file_path: &str,
        line_number: u32,
        side: FeedbackSide,
    ) -> Option<&IndexedHunk> {
        let file_index = self.files.get(file_path)?;
        for indexed_hunk in &file_index.all_hunks {
            let coords = indexed_hunk.coords;
            let hunk = &indexed_hunk.hunk;
            match side {
                FeedbackSide::Old => {
                    let old_end = coords.0 + hunk.source_length.saturating_sub(1) as u32;
                    if line_number >= coords.0 && line_number <= old_end {
                        return Some(indexed_hunk);
                    }
                }
                FeedbackSide::New => {
                    let new_end = coords.1 + hunk.target_length.saturating_sub(1) as u32;
                    if line_number >= coords.1 && line_number <= new_end {
                        return Some(indexed_hunk);
                    }
                }
            }
        }
        None
    }

    /// Renders a single hunk as a unified diff string.
    pub fn render_hunk_unified(hunk: &unidiff::Hunk, _coords: (u32, u32)) -> String {
        hunk.to_string()
    }

    /// Calculates the total number of additions and deletions for a set of `DiffRef`s.
    pub fn task_stats(&self, diff_refs: &[DiffRef]) -> Result<(u32, u32)> {
        let mut additions = 0;
        let mut deletions = 0;

        for diff_ref in diff_refs {
            let file_index =
                self.files
                    .get(&diff_ref.file)
                    .ok_or_else(|| DiffIndexError::FileNotFound {
                        file: diff_ref.file.clone(),
                    })?;

            for hunk_ref in &diff_ref.hunks {
                let indexed_hunk = file_index
                    .hunks
                    .get(&(hunk_ref.old_start, hunk_ref.new_start))
                    .ok_or_else(|| {
                        // Find the nearest hunk for better error reporting
                        let nearest = find_nearest_hunk(
                            &file_index.all_hunks,
                            (hunk_ref.old_start, hunk_ref.new_start),
                        );
                        DiffIndexError::NearestHunk {
                            file: diff_ref.file.clone(),
                            old_start: hunk_ref.old_start,
                            new_start: hunk_ref.new_start,
                            nearest,
                        }
                    })?;

                for line in indexed_hunk.hunk.lines() {
                    match line.line_type.as_str() {
                        unidiff::LINE_TYPE_ADDED => additions += 1,
                        unidiff::LINE_TYPE_REMOVED => deletions += 1,
                        _ => {}
                    }
                }
            }
        }
        Ok((additions, deletions))
    }

    /// Checks if a hunk exists in the specified file.
    pub fn validate_hunk_exists(&self, file_path: &str, hunk_ref: &HunkRef) -> Result<()> {
        let file_index = self
            .files
            .get(file_path)
            .ok_or_else(|| DiffIndexError::FileNotFound {
                file: file_path.to_string(),
            })?;

        file_index
            .hunks
            .get(&(hunk_ref.old_start, hunk_ref.new_start))
            .ok_or_else(|| {
                // Find the nearest hunk for better error reporting
                let nearest = find_nearest_hunk(
                    &file_index.all_hunks,
                    (hunk_ref.old_start, hunk_ref.new_start),
                );
                DiffIndexError::NearestHunk {
                    file: file_path.to_string(),
                    old_start: hunk_ref.old_start,
                    new_start: hunk_ref.new_start,
                    nearest,
                }
            })?;

        Ok(())
    }

    /// Checks if a file exists in the diff index.
    pub fn validate_file_exists(&self, file_path: &str) -> Result<()> {
        if self.files.contains_key(file_path) {
            Ok(())
        } else {
            Err(DiffIndexError::FileNotFound {
                file: file_path.to_string(),
            }
            .into())
        }
    }

    /// Generate a manifest of all hunks with their coordinates.
    /// This helps agents reference hunks accurately by providing exact coordinates to copy.
    pub fn generate_hunk_manifest(&self) -> String {
        let mut result = String::new();

        // Sort files for consistent ordering
        let mut sorted_files: Vec<_> = self.files.keys().collect();
        sorted_files.sort();

        for file_path in sorted_files {
            let file_index = self.files.get(file_path).unwrap();
            if file_index.all_hunks.is_empty() {
                continue;
            }

            result.push_str(&format!("{}:\n", file_path));

            for (idx, indexed_hunk) in file_index.all_hunks.iter().enumerate() {
                let hunk = &indexed_hunk.hunk;
                let coords = &indexed_hunk.coords;

                // Count additions and deletions in this hunk
                let mut adds = 0;
                let mut dels = 0;
                for line in hunk.lines() {
                    match line.line_type.as_str() {
                        unidiff::LINE_TYPE_ADDED => adds += 1,
                        unidiff::LINE_TYPE_REMOVED => dels += 1,
                        _ => {}
                    }
                }

                result.push_str(&format!(
                    "  H{}: {{ \"old_start\": {}, \"old_lines\": {}, \"new_start\": {}, \"new_lines\": {} }}  (+{}, -{})\n",
                    idx + 1,
                    coords.0,
                    hunk.source_length,
                    coords.1,
                    hunk.target_length,
                    adds,
                    dels
                ));
            }
        }

        result
    }

    /// Generate a JSON manifest of all hunks with their coordinates.
    /// This is machine-readable and can be copied directly into diff_refs.hunks.
    pub fn generate_hunk_manifest_json(&self) -> String {
        let mut manifest: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();

        let mut sorted_files: Vec<_> = self.files.keys().cloned().collect();
        sorted_files.sort();

        for file_path in sorted_files {
            let file_index = self.files.get(&file_path).unwrap();
            if file_index.all_hunks.is_empty() {
                continue;
            }

            let mut hunks = Vec::new();
            for indexed_hunk in &file_index.all_hunks {
                let hunk = &indexed_hunk.hunk;
                let coords = &indexed_hunk.coords;
                hunks.push(serde_json::json!({
                    "old_start": coords.0,
                    "old_lines": hunk.source_length,
                    "new_start": coords.1,
                    "new_lines": hunk.target_length
                }));
            }

            manifest.insert(file_path.clone(), hunks);
        }

        serde_json::to_string_pretty(&manifest).unwrap_or_default()
    }

    /// Renders a unified diff snippet for the given `DiffRef`s.
    /// Returns the diff string and a list of ordered file paths.
    pub fn render_unified_diff(&self, diff_refs: &[DiffRef]) -> Result<(String, Vec<String>)> {
        let mut result = String::new();
        let mut ordered_files = Vec::new();

        for diff_ref in diff_refs {
            let file_index =
                self.files
                    .get(&diff_ref.file)
                    .ok_or_else(|| DiffIndexError::FileNotFound {
                        file: diff_ref.file.clone(),
                    })?;

            // Build the header for this file
            let header = format!(
                "diff --git a/{} b/{}\n--- a/{}\n+++ b/{}\n",
                diff_ref.file, diff_ref.file, diff_ref.file, diff_ref.file
            );
            ordered_files.push(diff_ref.file.clone());
            result.push_str(&header);
            let hunks_to_render = if diff_ref.hunks.is_empty() {
                // If hunks are empty, render all available hunks for the file
                file_index
                    .all_hunks
                    .iter()
                    .map(|indexed_hunk| &indexed_hunk.hunk)
                    .collect()
            } else {
                // Otherwise, render only the specified hunks
                let mut hunks = Vec::new();
                for hunk_ref in &diff_ref.hunks {
                    let indexed_hunk = file_index
                        .hunks
                        .get(&(hunk_ref.old_start, hunk_ref.new_start))
                        .ok_or_else(|| {
                            // Find the nearest hunk for better error reporting
                            let nearest = find_nearest_hunk(
                                &file_index.all_hunks,
                                (hunk_ref.old_start, hunk_ref.new_start),
                            );
                            DiffIndexError::NearestHunk {
                                file: diff_ref.file.clone(),
                                old_start: hunk_ref.old_start,
                                new_start: hunk_ref.new_start,
                                nearest,
                            }
                        })?;
                    hunks.push(&indexed_hunk.hunk);
                }
                hunks
            };
            for hunk in hunks_to_render {
                let hunk_text = hunk.to_string();
                let ends_with_newline = hunk_text.ends_with('\n');
                result.push_str(&hunk_text);
                if !ends_with_newline {
                    result.push('\n');
                }
            }
        }

        Ok((result, ordered_files))
    }

    /// Parse hunk_id (e.g., "src/auth.rs#H3") into file path and 1-based hunk index.
    /// Returns (file_path, hunk_index) where hunk_index is 1-based.
    pub fn parse_hunk_id(&self, hunk_id: &str) -> Option<(String, usize)> {
        if let Some((file_path, suffix)) = hunk_id.rsplit_once('#')
            && let Some(h_num) = suffix.strip_prefix('H')
            && let Ok(hunk_idx) = h_num.parse::<usize>()
            && hunk_idx > 0
        {
            return Some((file_path.to_string(), hunk_idx));
        }
        None
    }

    pub fn walk_hunk_lines<F>(hunk: &unidiff::Hunk, coords: (u32, u32), mut f: F)
    where
        F: FnMut(usize, &unidiff::Line, Option<u32>, Option<u32>),
    {
        let mut old_line = coords.0;
        let mut new_line = coords.1;

        for (pos, line) in hunk.lines().iter().enumerate() {
            let (old_num, new_num) = match line.line_type.as_str() {
                unidiff::LINE_TYPE_ADDED => (None, Some(new_line)),
                unidiff::LINE_TYPE_REMOVED => (Some(old_line), None),
                _ => (Some(old_line), Some(new_line)),
            };

            f(pos, line, old_num, new_num);

            match line.line_type.as_str() {
                unidiff::LINE_TYPE_ADDED => {
                    new_line += 1;
                }
                unidiff::LINE_TYPE_REMOVED => {
                    old_line += 1;
                }
                _ => {
                    old_line += 1;
                    new_line += 1;
                }
            }
        }
    }

    /// Find line by exact content within a hunk, with per-side line numbers.
    /// Automatically strips leading `+` or `-` diff markers from input if needed.
    /// Also handles multi-line input by extracting the last line with a diff marker (+/-).
    pub fn find_line_by_content_with_numbers(
        &self,
        hunk_id: &str,
        content: &str,
    ) -> Option<LineLocation> {
        let (file_path, hunk_idx) = self.parse_hunk_id(hunk_id)?;
        let file_index = self.files.get(&file_path)?;
        let indexed_hunk = file_index.all_hunks.get(hunk_idx - 1)?;
        let hunk = &indexed_hunk.hunk;
        let coords = indexed_hunk.coords;

        let lines_to_try: Vec<String> = if content.contains('\n') {
            let mut candidates: Vec<String> = Vec::new();
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('+') || trimmed.starts_with('-') {
                    candidates.push(trimmed[1..].trim_end().to_string());
                }
            }
            if candidates.is_empty() {
                vec![content.lines().last()?.trim_end().to_string()]
            } else {
                candidates
            }
        } else {
            let trimmed = content.trim_end();
            if trimmed.starts_with('+') || trimmed.starts_with('-') {
                vec![trimmed[1..].to_string()]
            } else {
                vec![trimmed.to_string()]
            }
        };

        let mut found = None;
        for normalized in lines_to_try {
            Self::walk_hunk_lines(hunk, coords, |pos, line, old_num, new_num| {
                if found.is_some() {
                    return;
                }

                let line_content = line.value.trim_end();
                if line_content == normalized {
                    found = Some(LineLocation {
                        position_in_hunk: pos,
                        old_line_number: old_num,
                        new_line_number: new_num,
                        is_addition: line.line_type.as_str() == unidiff::LINE_TYPE_ADDED,
                        is_deletion: line.line_type.as_str() == unidiff::LINE_TYPE_REMOVED,
                    });
                }
            });
            if found.is_some() {
                break;
            }
        }

        found
    }

    /// Find line by line ID within a hunk (e.g., "L3" for the 3rd line).
    /// This is the simplest and most reliable way to reference lines.
    /// Line IDs are 1-based (L1, L2, L3...).
    pub fn find_line_by_id(&self, hunk_id: &str, line_id: &str) -> Option<LineLocation> {
        let (file_path, hunk_idx) = self.parse_hunk_id(hunk_id)?;
        let file_index = self.files.get(&file_path)?;
        let indexed_hunk = file_index.all_hunks.get(hunk_idx - 1)?;
        let hunk = &indexed_hunk.hunk;
        let coords = indexed_hunk.coords;

        // Parse line ID (e.g., "L3" -> 3, or just "3" -> 3)
        let line_id_trimmed = line_id.trim();
        let line_idx: usize = if let Some(num_str) = line_id_trimmed.strip_prefix('L') {
            num_str.parse().ok()?
        } else if let Some(num_str) = line_id_trimmed.strip_prefix('l') {
            num_str.parse().ok()?
        } else {
            line_id_trimmed.parse().ok()?
        };

        // Line IDs are 1-based, so L1 = index 0
        if line_idx == 0 {
            return None;
        }
        let target_pos = line_idx - 1;

        let mut found = None;
        Self::walk_hunk_lines(hunk, coords, |pos, line, old_num, new_num| {
            if pos == target_pos {
                found = Some(LineLocation {
                    position_in_hunk: pos,
                    old_line_number: old_num,
                    new_line_number: new_num,
                    is_addition: line.line_type.as_str() == unidiff::LINE_TYPE_ADDED,
                    is_deletion: line.line_type.as_str() == unidiff::LINE_TYPE_REMOVED,
                });
            }
        });

        found
    }

    /// Find line location for a given line number and side.
    pub fn find_line_location(
        &self,
        file_path: &str,
        line_number: u32,
        side: FeedbackSide,
    ) -> Option<LineLocation> {
        let file_index = self.files.get(file_path)?;

        for indexed_hunk in &file_index.all_hunks {
            let hunk = &indexed_hunk.hunk;
            let coords = indexed_hunk.coords;

            let mut found = None;
            Self::walk_hunk_lines(hunk, coords, |pos, line, old_num, new_num| {
                if found.is_some() {
                    return;
                }

                let match_num = match side {
                    FeedbackSide::Old => old_num,
                    FeedbackSide::New => new_num,
                };
                if match_num == Some(line_number) {
                    found = Some(LineLocation {
                        position_in_hunk: pos,
                        old_line_number: old_num,
                        new_line_number: new_num,
                        is_addition: line.line_type.as_str() == unidiff::LINE_TYPE_ADDED,
                        is_deletion: line.line_type.as_str() == unidiff::LINE_TYPE_REMOVED,
                    });
                }
            });

            if let Some(location) = found {
                return Some(location);
            }
        }

        None
    }

    /// Find the position in the overall diff for a given line in the new file.
    /// This position is used by GitHub for review comments.
    pub fn find_position_in_diff(
        &self,
        file_path: &str,
        line_number: u32,
        side: FeedbackSide,
    ) -> Option<usize> {
        let file_index = self.files.get(file_path)?;
        let mut current_pos = 0;

        for indexed_hunk in &file_index.all_hunks {
            // Each hunk starts with a header line (@@) which is position 1 relative to the first hunk
            current_pos += 1;
            let header_pos = current_pos;

            let hunk = &indexed_hunk.hunk;
            let coords = indexed_hunk.coords;

            let mut found_pos = None;
            Self::walk_hunk_lines(hunk, coords, |pos, _line, old_num, new_num| {
                let match_num = match side {
                    FeedbackSide::Old => old_num,
                    FeedbackSide::New => new_num,
                };
                if match_num == Some(line_number) {
                    found_pos = Some(header_pos + pos + 1);
                }
            });

            if let Some(pos) = found_pos {
                return Some(pos);
            }

            // Advance current_pos by the number of lines in this hunk
            current_pos += hunk.lines().len();
        }

        None
    }

    /// Find line by exact content within a hunk.
    /// Returns (line_number_in_file, position_in_hunk_lines_array).
    pub fn find_line_by_content(&self, hunk_id: &str, content: &str) -> Option<(u32, usize)> {
        let line = self.find_line_by_content_with_numbers(hunk_id, content)?;
        let line_number = line.new_line_number.or(line.old_line_number)?;
        Some((line_number, line.position_in_hunk))
    }

    /// Get hunk coordinates from hunk ID.
    pub fn get_hunk_coords(&self, hunk_id: &str) -> Option<HunkRef> {
        let (file_path, hunk_idx) = self.parse_hunk_id(hunk_id)?;
        let file_index = self.files.get(&file_path)?;

        let indexed_hunk = file_index.all_hunks.get(hunk_idx - 1)?;
        let hunk = &indexed_hunk.hunk;
        let coords = indexed_hunk.coords;

        Some(HunkRef {
            old_start: coords.0,
            old_lines: hunk.source_length as u32,
            new_start: coords.1,
            new_lines: hunk.target_length as u32,
        })
    }

    /// Generate a unified, agent-friendly manifest with copy-paste ready content.
    /// This is the ONLY manifest needed - consolidates all previous formats.
    /// Now includes line IDs (L1, L2, etc.) for easy feedback targeting.
    pub fn generate_unified_manifest(&self) -> String {
        let mut result = String::new();

        result.push_str("# Unified Diff Manifest\n\n");
        result.push_str("## How to Use\n\n");
        result.push_str("**For tasks:** Use `hunk_ids` array with IDs like `\"src/file.rs#H1\"`\n");
        result.push_str("**For feedback:** Use `hunk_id` + `line_id` (e.g., `\"L3\"`)\n\n");

        let mut sorted_files: Vec<_> = self.files.keys().cloned().collect();
        sorted_files.sort();

        for file_path in sorted_files {
            let file_index = self.files.get(&file_path).unwrap();
            if file_index.all_hunks.is_empty() {
                continue;
            }

            result.push_str(&format!("# {}\n", file_path));
            result.push_str(&format!(
                "**hunk_ids:** [{}]\n\n",
                file_index
                    .all_hunks
                    .iter()
                    .enumerate()
                    .map(|(idx, _)| format!("\"{}#H{}\"", file_path, idx + 1))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));

            for (idx, indexed_hunk) in file_index.all_hunks.iter().enumerate() {
                let hunk = &indexed_hunk.hunk;
                let hunk_id = format!("{}#H{}", file_path, idx + 1);
                let coords = indexed_hunk.coords;

                let old_end = coords.0 + hunk.source_length.saturating_sub(1) as u32;
                let new_end = coords.1 + hunk.target_length.saturating_sub(1) as u32;

                result.push_str(&format!("## {}\n", hunk_id));
                result.push_str(&format!(
                    "Old: {}-{} | New: {}-{}\n",
                    coords.0, old_end, coords.1, new_end
                ));
                result.push_str("```\n");

                for (line_idx, line) in hunk.lines().iter().enumerate() {
                    let display_line = line.value.trim_end();
                    let prefix = match line.line_type.as_str() {
                        unidiff::LINE_TYPE_ADDED => "+",
                        unidiff::LINE_TYPE_REMOVED => "-",
                        _ => " ",
                    };
                    // Line IDs are 1-based (L1, L2, L3...)
                    result.push_str(&format!(
                        "{} L{:<2} | {}\n",
                        prefix,
                        line_idx + 1,
                        display_line
                    ));
                }
                result.push_str("```\n\n");
            }
        }

        result
    }

    /// Generate JSON manifest with copy-paste ready hunk IDs.
    pub fn generate_hunk_ids_manifest(&self) -> serde_json::Value {
        let mut manifest: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();

        let mut sorted_files: Vec<_> = self.files.keys().cloned().collect();
        sorted_files.sort();

        for file_path in sorted_files {
            let file_index = self.files.get(&file_path).unwrap();
            if file_index.all_hunks.is_empty() {
                continue;
            }

            let mut hunks = Vec::new();
            for (idx, indexed_hunk) in file_index.all_hunks.iter().enumerate() {
                let hunk = &indexed_hunk.hunk;
                let coords = indexed_hunk.coords;
                let hunk_id = format!("{}#H{}", file_path, idx + 1);

                hunks.push(serde_json::json!({
                    "hunk_id": hunk_id,
                    "old_start": coords.0,
                    "old_lines": hunk.source_length,
                    "new_start": coords.1,
                    "new_lines": hunk.target_length
                }));
            }

            manifest.insert(file_path.clone(), hunks);
        }

        serde_json::to_value(manifest).unwrap_or_default()
    }

    /// Find all matching lines across the entire diff for content search.
    pub fn find_all_by_content(&self, content: &str) -> Vec<HunkMatch> {
        let mut matches = Vec::new();

        for (file_path, file_index) in &self.files {
            for (hunk_idx, indexed_hunk) in file_index.all_hunks.iter().enumerate() {
                let hunk = &indexed_hunk.hunk;
                let hunk_id = format!("{}#H{}", file_path, hunk_idx + 1);
                let coords = indexed_hunk.coords;

                let mut line_matches = Vec::new();
                Self::walk_hunk_lines(hunk, coords, |pos, line, old_num, new_num| {
                    let line_content = line.value.trim_end();
                    if line_content.contains(content) {
                        let line_number = new_num.or(old_num).unwrap_or(coords.1);
                        line_matches.push(LineMatch {
                            line_number,
                            line_content: line_content.to_string(),
                            position_in_hunk: pos,
                            is_addition: line.line_type.as_str() == unidiff::LINE_TYPE_ADDED,
                            is_deletion: line.line_type.as_str() == unidiff::LINE_TYPE_REMOVED,
                        });
                    }
                });

                if !line_matches.is_empty() {
                    matches.push(HunkMatch {
                        hunk_id,
                        file_path: file_path.clone(),
                        hunk_ref: HunkRef {
                            old_start: coords.0,
                            old_lines: hunk.source_length as u32,
                            new_start: coords.1,
                            new_lines: hunk.target_length as u32,
                        },
                        lines: line_matches,
                    });
                }
            }
        }

        matches
    }

    /// Get all available hunk IDs for a file.
    pub fn get_hunk_ids_for_file(&self, file_path: &str) -> Vec<String> {
        if let Some(file_index) = self.files.get(file_path) {
            file_index
                .all_hunks
                .iter()
                .enumerate()
                .map(|(idx, _)| format!("{}#H{}", file_path, idx + 1))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get all file paths in the diff.
    pub fn get_all_file_paths(&self) -> Vec<String> {
        self.files.keys().cloned().collect()
    }

    /// Calculate total diff statistics (total files, hunks, additions, deletions).
    pub fn total_stats(&self) -> (usize, usize, usize, usize) {
        let mut total_files = 0;
        let mut total_hunks = 0;
        let mut total_additions = 0;
        let mut total_deletions = 0;

        for file_index in self.files.values() {
            if !file_index.all_hunks.is_empty() {
                total_files += 1;
            }
            for indexed_hunk in &file_index.all_hunks {
                total_hunks += 1;
                for line in indexed_hunk.hunk.lines() {
                    match line.line_type.as_str() {
                        unidiff::LINE_TYPE_ADDED => total_additions += 1,
                        unidiff::LINE_TYPE_REMOVED => total_deletions += 1,
                        _ => {}
                    }
                }
            }
        }

        (total_files, total_hunks, total_additions, total_deletions)
    }

    /// Generate a compact manifest for large diffs.
    /// This provides summary stats and per-file metadata without line content,
    /// allowing agents to use MCP tools for on-demand content retrieval.
    pub fn generate_compact_manifest(&self) -> String {
        let (total_files, total_hunks, total_additions, total_deletions) = self.total_stats();

        let mut result = String::new();
        result.push_str("# Large Diff Manifest\n\n");
        result.push_str("## Summary\n");
        result.push_str(&format!(
            "- **Files changed**: {}\n- **Total hunks**: {}\n- **Lines added**: +{}\n- **Lines removed**: -{}\n\n",
            total_files, total_hunks, total_additions, total_deletions
        ));

        // Collect file stats and sort by total changes (largest first)
        let mut file_stats: Vec<(String, usize, usize, usize, Vec<String>)> = Vec::new();

        for (file_path, file_index) in &self.files {
            if file_index.all_hunks.is_empty() {
                continue;
            }

            let mut file_adds = 0;
            let mut file_dels = 0;
            let mut hunk_ids = Vec::new();

            for (idx, indexed_hunk) in file_index.all_hunks.iter().enumerate() {
                let hunk_id = format!("{}#H{}", file_path, idx + 1);
                hunk_ids.push(hunk_id);

                for line in indexed_hunk.hunk.lines() {
                    match line.line_type.as_str() {
                        unidiff::LINE_TYPE_ADDED => file_adds += 1,
                        unidiff::LINE_TYPE_REMOVED => file_dels += 1,
                        _ => {}
                    }
                }
            }

            file_stats.push((
                file_path.clone(),
                file_adds,
                file_dels,
                file_index.all_hunks.len(),
                hunk_ids,
            ));
        }

        // Sort by total changes (additions + deletions), largest first
        file_stats.sort_by(|a, b| (b.1 + b.2).cmp(&(a.1 + a.2)));

        result.push_str("## Files (sorted by change size)\n\n");
        result.push_str("Use `get_hunk` or `get_file_hunks` tools to retrieve content.\n\n");

        for (file_path, adds, dels, hunk_count, hunk_ids) in file_stats {
            result.push_str(&format!("### {}\n", file_path));
            result.push_str(&format!(
                "- Changes: +{}, -{} ({} hunks)\n",
                adds, dels, hunk_count
            ));
            result.push_str(&format!("- Hunk IDs: {}\n\n", hunk_ids.join(", ")));
        }

        result.push_str("## Available Tools\n\n");
        result.push_str("- `get_hunk { hunk_id: \"path/file.rs#H1\" }` - Get content of a single hunk\n");
        result.push_str("- `get_file_hunks { file_path: \"path/file.rs\" }` - Get all hunks for a file\n");
        result.push_str("- `search_diff { pattern: \"keyword\" }` - Search across all diff content\n");
        result.push_str("- `list_diff_files` - List all changed files\n");

        result
    }

    /// Get line content and metadata for all lines in a hunk.
    pub fn get_hunk_lines(&self, hunk_id: &str) -> Option<Vec<(usize, String, bool, bool)>> {
        let lines = self.get_hunk_lines_with_numbers(hunk_id)?;
        Some(
            lines
                .into_iter()
                .map(|line| {
                    (
                        line.position_in_hunk,
                        line.content,
                        line.is_addition,
                        line.is_deletion,
                    )
                })
                .collect(),
        )
    }

    /// Get line content, metadata, and per-side line numbers for all lines in a hunk.
    pub fn get_hunk_lines_with_numbers(&self, hunk_id: &str) -> Option<Vec<HunkLineInfo>> {
        let (file_path, hunk_idx) = self.parse_hunk_id(hunk_id)?;
        let file_index = self.files.get(&file_path)?;
        let indexed_hunk = file_index.all_hunks.get(hunk_idx - 1)?;
        let hunk = &indexed_hunk.hunk;
        let coords = indexed_hunk.coords;

        let mut lines = Vec::new();
        Self::walk_hunk_lines(hunk, coords, |pos, line, old_num, new_num| {
            let is_add = line.line_type.as_str() == unidiff::LINE_TYPE_ADDED;
            let is_del = line.line_type.as_str() == unidiff::LINE_TYPE_REMOVED;
            lines.push(HunkLineInfo {
                position_in_hunk: pos,
                content: line.value.trim_end().to_string(),
                is_addition: is_add,
                is_deletion: is_del,
                old_line_number: old_num,
                new_line_number: new_num,
            });
        });

        Some(lines)
    }

    /// Check if a line exists in any hunk of a file, for a given side.
    pub fn line_exists_in_file(&self, file_path: &str, line: u32, side: FeedbackSide) -> bool {
        let file_index = match self.files.get(file_path) {
            Some(f) => f,
            None => return false,
        };

        for indexed_hunk in &file_index.all_hunks {
            let hunk = &indexed_hunk.hunk;
            let coords = indexed_hunk.coords;

            match side {
                FeedbackSide::Old => {
                    let old_end = coords.0 + hunk.source_length.saturating_sub(1) as u32;
                    if line >= coords.0 && line <= old_end {
                        return true;
                    }
                }
                FeedbackSide::New => {
                    let new_end = coords.1 + hunk.target_length.saturating_sub(1) as u32;
                    if line >= coords.1 && line <= new_end {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Get hunk ranges for a file (for error messages).
    pub fn get_hunk_ranges(&self, file_path: &str) -> (Vec<LineRange>, Vec<LineRange>) {
        let file_index = match self.files.get(file_path) {
            Some(f) => f,
            None => return (Vec::new(), Vec::new()),
        };

        let old_ranges: Vec<(u32, u32)> = file_index
            .all_hunks
            .iter()
            .map(|ih| {
                let coords = ih.coords;
                let old_end = coords.0 + ih.hunk.source_length as u32 - 1;
                (coords.0, old_end)
            })
            .collect();

        let new_ranges: Vec<(u32, u32)> = file_index
            .all_hunks
            .iter()
            .map(|ih| {
                let coords = ih.coords;
                let new_end = coords.1 + ih.hunk.target_length as u32 - 1;
                (coords.1, new_end)
            })
            .collect();

        (old_ranges, new_ranges)
    }

    /// Get the formatted content of a hunk given its coordinates.
    pub fn get_hunk_content_by_coords(
        &self,
        file_path: &str,
        old_start: u32,
        new_start: u32,
    ) -> Option<String> {
        let file_index = self.files.get(file_path)?;
        let indexed_hunk = file_index.hunks.get(&(old_start, new_start))?;
        let hunk = &indexed_hunk.hunk;

        let mut content = String::new();
        for line in hunk.lines() {
            let prefix = match line.line_type.as_str() {
                unidiff::LINE_TYPE_ADDED => "+",
                unidiff::LINE_TYPE_REMOVED => "-",
                _ => " ",
            };
            content.push_str(&format!("{}{}\n", prefix, line.value.trim_end()));
        }

        Some(content)
    }
}

/// Helper function to find the nearest hunk to the given coordinates
fn find_nearest_hunk(all_hunks: &[IndexedHunk], target: (u32, u32)) -> Option<(u32, u32)> {
    if all_hunks.is_empty() {
        return None;
    }

    let mut nearest = None;
    let mut min_distance = u32::MAX;

    for indexed_hunk in all_hunks {
        let coords = indexed_hunk.coords;
        // Calculate a simple distance metric
        let distance =
            (target.0 as i32 - coords.0 as i32).abs() + (target.1 as i32 - coords.1 as i32).abs();
        if distance < min_distance as i32 {
            min_distance = distance as u32;
            nearest = Some(coords);
        }
    }

    nearest
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_DIFF: &str = r#"diff --git a/src/main.rs b/src/main.rs
index 0123456..789abcd 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,5 @@
 fn main() {
-    println!("Hello, world!");
+    println!("Hello, Gemini!");
     println!("Another line");
 }

diff --git a/src/lib.rs b/src/lib.rs
new file mode 100644
index 0000000..abcdefg
--- /dev/null
+++ b/src/lib.rs
@@ -0,0 +1,3 @@
+pub fn add(a: i32, b: i32) -> i32 {
+    a + b
+}
"#;

    #[test]
    fn test_find_line_by_content() {
        let index = DiffIndex::new(TEST_DIFF).unwrap();

        let result = index.find_line_by_content("src/main.rs#H1", "fn main() {");
        assert!(result.is_some());
        let (_line_num, pos) = result.unwrap();
        assert_eq!(pos, 0);

        let result =
            index.find_line_by_content("src/main.rs#H1", "    println!(\"Hello, Gemini!\");");
        assert!(result.is_some());
        let (_line_num, pos) = result.unwrap();
        assert_eq!(
            pos, 2,
            "Added line should be at position 2 (after deleted line)"
        );

        let result =
            index.find_line_by_content("src/lib.rs#H1", "pub fn add(a: i32, b: i32) -> i32 {");
        assert!(result.is_some());
        let (_line_num, pos) = result.unwrap();
        assert_eq!(pos, 0);

        assert!(
            index
                .find_line_by_content("src/main.rs#H1", "nonexistent line")
                .is_none()
        );
        assert!(
            index
                .find_line_by_content("invalid#H1", "fn main()")
                .is_none()
        );
    }

    #[test]
    fn test_generate_hunk_manifest_json() {
        let index = DiffIndex::new(TEST_DIFF).unwrap();
        let manifest = index.generate_hunk_manifest_json();
        assert!(manifest.contains("\"src/main.rs\""));
        assert!(manifest.contains("\"old_start\": 1"));
        assert!(manifest.contains("\"new_start\": 1"));
    }

    #[test]
    fn test_task_stats() {
        let index = DiffIndex::new(TEST_DIFF).unwrap();
        let diff_refs = vec![
            DiffRef {
                file: "src/main.rs".to_string(),
                hunks: vec![HunkRef {
                    old_start: 1,
                    old_lines: 5,
                    new_start: 1,
                    new_lines: 5,
                }],
            },
            DiffRef {
                file: "src/lib.rs".to_string(),
                hunks: vec![HunkRef {
                    old_start: 0,
                    old_lines: 0,
                    new_start: 1,
                    new_lines: 3,
                }],
            },
        ];

        let (additions, deletions) = index.task_stats(&diff_refs).unwrap();
        assert_eq!(additions, 4); // 1 in main.rs, 3 in lib.rs
        assert_eq!(deletions, 1); // 1 in main.rs
    }

    #[test]
    fn test_render_unified_diff() {
        let index = DiffIndex::new(TEST_DIFF).unwrap();
        let diff_refs = vec![DiffRef {
            file: "src/main.rs".to_string(),
            hunks: vec![HunkRef {
                old_start: 1,
                old_lines: 5,
                new_start: 1,
                new_lines: 5,
            }],
        }];

        let (diff_text, files) = index.render_unified_diff(&diff_refs).unwrap();
        assert_eq!(files, vec!["src/main.rs"]);
        assert!(diff_text.contains("diff --git a/src/main.rs b/src/main.rs"));
        assert!(diff_text.contains("@@ -1,5 +1,5 @@"));
        assert!(diff_text.contains("-    println!(\"Hello, world!\");"));
        assert!(diff_text.contains("+    println!(\"Hello, Gemini!\");"));
        assert!(!diff_text.contains("diff --git a/src/lib.rs b/src/lib.rs"));
    }

    #[test]
    fn test_render_multiple_hunks() {
        let diff_with_multiple_hunks = r###"diff --git a/file.txt b/file.txt
--- a/file.txt
+++ b/file.txt
@@ -1,3 +1,3 @@
 line 1
-line 2
+line 2 changed
 line 3
@@ -10,3 +10,3 @@
 line 10
-line 11
+line 11 changed
 line 12
"###;

        let index = DiffIndex::new(diff_with_multiple_hunks).unwrap();
        let diff_refs = vec![DiffRef {
            file: "file.txt".to_string(),
            hunks: vec![HunkRef {
                old_start: 10,
                old_lines: 3,
                new_start: 10,
                new_lines: 3,
            }],
        }];

        let (diff_text, _) = index.render_unified_diff(&diff_refs).unwrap();
        assert!(diff_text.contains("@@ -10,3 +10,3 @@"));
        assert!(!diff_text.contains("@@ -1,3 +1,3 @@"));
    }

    #[test]
    fn test_diff_index_error_display() {
        let err = DiffIndexError::FileNotFound {
            file: "missing.rs".into(),
        };
        assert!(err.to_string().contains("missing.rs"));
        assert!(err.to_json().contains("FileNotFound"));
    }

    #[test]
    fn test_parse_hunk_id() {
        let index = DiffIndex::new(TEST_DIFF).unwrap();

        let (file, idx) = index.parse_hunk_id("src/main.rs#H1").unwrap();
        assert_eq!(file, "src/main.rs");
        assert_eq!(idx, 1);

        let (file, idx) = index.parse_hunk_id("src/lib.rs#H1").unwrap();
        assert_eq!(file, "src/lib.rs");
        assert_eq!(idx, 1);

        assert!(index.parse_hunk_id("src/main.rs").is_none());
        assert!(index.parse_hunk_id("src/main.rs#1").is_none());
        assert!(index.parse_hunk_id("src/main.rs#H0").is_none());
        assert!(index.parse_hunk_id("invalid").is_none());
    }

    #[test]
    fn test_get_hunk_coords() {
        let index = DiffIndex::new(TEST_DIFF).unwrap();

        let coords = index.get_hunk_coords("src/main.rs#H1").unwrap();
        assert_eq!(coords.old_start, 1);
        assert_eq!(coords.new_start, 1);
        assert!(coords.old_lines > 0);
        assert!(coords.new_lines > 0);

        assert!(index.get_hunk_coords("invalid#H1").is_none());
        assert!(index.get_hunk_coords("src/main.rs#H99").is_none());
    }

    #[test]
    fn test_generate_unified_manifest() {
        let index = DiffIndex::new(TEST_DIFF).unwrap();
        let manifest = index.generate_unified_manifest();

        assert!(manifest.contains("# src/main.rs"));
        assert!(manifest.contains("## src/main.rs#H1"));
        // New format includes line IDs (L1, L2, etc.)
        assert!(manifest.contains("L1"));
        // Old/New line ranges are now on same line
        assert!(manifest.contains("Old:"));
        assert!(manifest.contains("New:"));
    }

    #[test]
    fn test_generate_hunk_ids_manifest() {
        let index = DiffIndex::new(TEST_DIFF).unwrap();
        let manifest = index.generate_hunk_ids_manifest();

        let main_hunks = manifest.get("src/main.rs").unwrap().as_array().unwrap();
        assert_eq!(main_hunks.len(), 1);
        assert_eq!(main_hunks[0]["hunk_id"], "src/main.rs#H1");
        assert_eq!(main_hunks[0]["old_start"], 1);
        assert_eq!(main_hunks[0]["new_start"], 1);

        let lib_hunks = manifest.get("src/lib.rs").unwrap().as_array().unwrap();
        assert_eq!(lib_hunks.len(), 1);
        assert_eq!(lib_hunks[0]["hunk_id"], "src/lib.rs#H1");
    }

    #[test]
    fn test_find_all_by_content() {
        let diff = r#"diff --git a/file.txt b/file.txt
--- a/file.txt
+++ b/file.txt
@@ -1,5 +1,5 @@
 context line
-context line 2
+context line 2 changed
 context line 3
@@ -10,5 +10,5 @@
 context line
-context line 2
+context line 2 changed
 context line 3
"#;

        let index = DiffIndex::new(diff).unwrap();
        let matches = index.find_all_by_content("context line 2 changed");

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].hunk_id, "file.txt#H1");
        assert_eq!(matches[1].hunk_id, "file.txt#H2");
    }

    #[test]
    fn test_get_hunk_ids_for_file() {
        let index = DiffIndex::new(TEST_DIFF).unwrap();

        let ids = index.get_hunk_ids_for_file("src/main.rs");
        assert_eq!(ids, vec!["src/main.rs#H1"]);

        let ids = index.get_hunk_ids_for_file("src/lib.rs");
        assert_eq!(ids, vec!["src/lib.rs#H1"]);

        let ids = index.get_hunk_ids_for_file("nonexistent");
        assert!(ids.is_empty());
    }

    #[test]
    fn test_find_line_by_content_complex_hunk() {
        // Complex hunk with multiple hunks and mixed line types
        let diff = r##"diff --git a/complex.rs b/complex.rs
--- a/complex.rs
+++ b/complex.rs
@@ -10,8 +10,8 @@
 context line 1
-context line 2
+added line 2
 context line 3
-context line 4
+added line 4
 context line 5
@@ -20,5 +20,5 @@
 context line
-removed line
+added line
 another context
diff --git a/trailer.rs b/trailer.rs
new file mode 100644
index 0000000..abcdefg
--- /dev/null
+++ b/trailer.rs
@@ -0,0 +1 @@
+trailer
"##;

        let index = DiffIndex::new(diff).unwrap();

        // Test first hunk: @@ -10,8 +10,8 @@
        // In unified diff format, removals come before additions:
        // Pos:  0        1 (rem)    2 (add)    3        4 (rem)    5 (add)    6
        //
        // New file line numbers:
        // - "context line 1" at pos 0: new = 10
        // - "added line 2" at pos 2: new = 11
        // - "context line 3" at pos 3: new = 12
        // - "added line 4" at pos 5: new = 13
        // - "context line 5" at pos 6: new = 14

        let result = index
            .find_line_by_content("complex.rs#H1", "context line 1")
            .unwrap();
        assert_eq!(result.0, 10);
        assert_eq!(result.1, 0);

        let result = index
            .find_line_by_content("complex.rs#H1", "added line 2")
            .unwrap();
        assert_eq!(result.0, 11);
        assert_eq!(result.1, 2);

        let result = index
            .find_line_by_content("complex.rs#H1", "context line 3")
            .unwrap();
        assert_eq!(result.0, 12);
        assert_eq!(result.1, 3);

        let result = index
            .find_line_by_content("complex.rs#H1", "added line 4")
            .unwrap();
        assert_eq!(result.0, 13);
        assert_eq!(result.1, 5);

        let result = index
            .find_line_by_content("complex.rs#H1", "context line 5")
            .unwrap();
        assert_eq!(result.0, 14);
        assert_eq!(result.1, 6);

        // Test second hunk: @@ -20,5 +20,5 @@
        // Pos:  0        1 (rem)    2 (add)    3
        //
        // New file line numbers:
        // - "context line" at pos 0: new = 20
        // - "added line" at pos 2: new = 21
        // - "another context" at pos 3: new = 22

        let result = index
            .find_line_by_content("complex.rs#H2", "context line")
            .unwrap();
        assert_eq!(result.0, 20);
        assert_eq!(result.1, 0);

        let result = index
            .find_line_by_content("complex.rs#H2", "added line")
            .unwrap();
        assert_eq!(result.0, 21);
        assert_eq!(result.1, 2);

        let result = index
            .find_line_by_content("complex.rs#H2", "another context")
            .unwrap();
        assert_eq!(result.0, 22);
        assert_eq!(result.1, 3);
    }

    #[test]
    fn test_find_line_by_content_with_numbers_divergent_sides() {
        let diff = r#"diff --git a/shift.rs b/shift.rs
--- a/shift.rs
+++ b/shift.rs
@@ -1,4 +1,5 @@
 line1
+inserted
 line2
 line3
 line4
@@ -10,5 +10,4 @@
 line10
-line11
 line12
 line13
 line14
"#;

        let index = DiffIndex::new(diff).unwrap();

        // H1: added line shifts new line numbers forward.
        let line = index
            .find_line_by_content_with_numbers("shift.rs#H1", "inserted")
            .unwrap();
        assert_eq!(line.old_line_number, None);
        assert_eq!(line.new_line_number, Some(2));
        assert_eq!(line.position_in_hunk, 1);

        let line = index
            .find_line_by_content_with_numbers("shift.rs#H1", "line2")
            .unwrap();
        assert_eq!(line.old_line_number, Some(2));
        assert_eq!(line.new_line_number, Some(3));
        assert_eq!(line.position_in_hunk, 2);

        // H2: removed line shifts new line numbers backward.
        let line = index
            .find_line_by_content_with_numbers("shift.rs#H2", "line11")
            .unwrap();
        assert_eq!(line.old_line_number, Some(11));
        assert_eq!(line.new_line_number, None);
        assert_eq!(line.position_in_hunk, 1);

        let line = index
            .find_line_by_content_with_numbers("shift.rs#H2", "line12")
            .unwrap();
        assert_eq!(line.old_line_number, Some(12));
        assert_eq!(line.new_line_number, Some(11));
        assert_eq!(line.position_in_hunk, 2);

        // find_line_by_content prefers new line numbers when available.
        let result = index.find_line_by_content("shift.rs#H1", "line2").unwrap();
        assert_eq!(result.0, 3);
        let result = index.find_line_by_content("shift.rs#H2", "line12").unwrap();
        assert_eq!(result.0, 11);

        // Removed lines fall back to old line numbers.
        let result = index.find_line_by_content("shift.rs#H2", "line11").unwrap();
        assert_eq!(result.0, 11);
    }

    #[test]
    fn test_find_line_by_content_with_numbers_strips_diff_markers() {
        let diff = r#"diff --git a/test.rs b/test.rs
--- a/test.rs
+++ b/test.rs
@@ -1,5 +1,5 @@
 context
-removed line
+added line
another context
"#;

        let index = DiffIndex::new(diff).unwrap();

        // Should find the line even when input has + prefix (common agent mistake)
        let line = index
            .find_line_by_content_with_numbers("test.rs#H1", "+added line")
            .unwrap();
        assert_eq!(line.new_line_number, Some(2));
        assert_eq!(line.position_in_hunk, 2);

        // Should also work with - prefix
        let line = index
            .find_line_by_content_with_numbers("test.rs#H1", "-removed line")
            .unwrap();
        assert_eq!(line.old_line_number, Some(2));
        assert_eq!(line.position_in_hunk, 1);

        // Original content without prefix should still work
        let line = index
            .find_line_by_content_with_numbers("test.rs#H1", "added line")
            .unwrap();
        assert_eq!(line.new_line_number, Some(2));
    }

    #[test]
    fn test_find_line_by_id() {
        let index = DiffIndex::new(TEST_DIFF).unwrap();

        // L1 should be the first line in the hunk
        let line = index.find_line_by_id("src/main.rs#H1", "L1").unwrap();
        assert_eq!(line.position_in_hunk, 0);

        // Should work with lowercase 'l'
        let line = index.find_line_by_id("src/main.rs#H1", "l1").unwrap();
        assert_eq!(line.position_in_hunk, 0);

        // Should work with just the number
        let line = index.find_line_by_id("src/main.rs#H1", "2").unwrap();
        assert_eq!(line.position_in_hunk, 1);

        // Invalid line ID should return None
        assert!(index.find_line_by_id("src/main.rs#H1", "L999").is_none());
        assert!(index.find_line_by_id("src/main.rs#H1", "L0").is_none());
        assert!(index.find_line_by_id("nonexistent#H1", "L1").is_none());
    }

    #[test]
    fn test_total_stats() {
        let index = DiffIndex::new(TEST_DIFF).unwrap();
        let (files, hunks, additions, deletions) = index.total_stats();

        assert_eq!(files, 2); // src/main.rs and src/lib.rs
        assert_eq!(hunks, 2); // one hunk in each file
        assert_eq!(additions, 4); // 1 in main.rs, 3 in lib.rs
        assert_eq!(deletions, 1); // 1 in main.rs
    }

    #[test]
    fn test_generate_compact_manifest() {
        let index = DiffIndex::new(TEST_DIFF).unwrap();
        let manifest = index.generate_compact_manifest();

        // Check summary section
        assert!(manifest.contains("# Large Diff Manifest"));
        assert!(manifest.contains("## Summary"));
        assert!(manifest.contains("**Files changed**: 2"));
        assert!(manifest.contains("**Total hunks**: 2"));
        assert!(manifest.contains("**Lines added**: +4"));
        assert!(manifest.contains("**Lines removed**: -1"));

        // Check that files are listed
        assert!(manifest.contains("### src/lib.rs") || manifest.contains("### src/main.rs"));

        // Check that hunk IDs are present
        assert!(manifest.contains("src/main.rs#H1"));
        assert!(manifest.contains("src/lib.rs#H1"));

        // Check available tools section
        assert!(manifest.contains("## Available Tools"));
        assert!(manifest.contains("get_hunk"));
        assert!(manifest.contains("get_file_hunks"));
        assert!(manifest.contains("search_diff"));
        assert!(manifest.contains("list_diff_files"));
    }

    #[test]
    fn test_compact_manifest_sorted_by_change_size() {
        // Create a diff where one file has more changes than another
        let diff = r#"diff --git a/small.rs b/small.rs
--- a/small.rs
+++ b/small.rs
@@ -1,2 +1,2 @@
 line1
-old
+new
diff --git a/large.rs b/large.rs
--- a/large.rs
+++ b/large.rs
@@ -1,5 +1,8 @@
 line1
-old1
-old2
-old3
+new1
+new2
+new3
+new4
+new5
 line6
"#;
        let index = DiffIndex::new(diff).unwrap();
        let manifest = index.generate_compact_manifest();

        // large.rs should appear before small.rs (sorted by change size)
        let large_pos = manifest.find("### large.rs").unwrap();
        let small_pos = manifest.find("### small.rs").unwrap();
        assert!(
            large_pos < small_pos,
            "large.rs should appear before small.rs in the manifest"
        );
    }
}
