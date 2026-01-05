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
struct IndexedHunk {
    hunk: Hunk,
    /// The coordinates of the hunk (old_start, new_start)
    coords: (u32, u32),
}

/// An index for a single file within a larger diff.
#[derive(Debug, Clone)]
struct FileIndex {
    /// A map from (old_start, new_start) to the indexed hunk.
    hunks: HashMap<(u32, u32), IndexedHunk>,
    /// A list of all available hunks in order.
    all_hunks: Vec<IndexedHunk>,
}

/// An index for a unified diff, allowing for efficient queries.
#[derive(Debug, Clone)]
pub struct DiffIndex {
    files: HashMap<String, FileIndex>,
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
            let mut hunks_by_id = HashMap::new();
            let mut all_hunks = Vec::new();

            // Convert file path to a safe identifier format for use in IDs
            let safe_file_path = file_path.replace("/", "_").replace("-", "_");
            for (hunk_idx, hunk) in file.hunks().iter().enumerate() {
                let coords = (hunk.source_start as u32, hunk.target_start as u32);

                // Create a hunk ID that's descriptive based on file and sequential index
                // Format: file_path#Hn where n is the sequential hunk number in the file (1-indexed)
                let hunk_id = format!("{}#H{}", safe_file_path, hunk_idx);

                let indexed_hunk = IndexedHunk {
                    hunk: hunk.clone(),
                    coords,
                };

                hunks.insert(coords, indexed_hunk.clone());
                hunks_by_id.insert(hunk_id, indexed_hunk.clone());
                all_hunks.push(indexed_hunk);
            }

            files.insert(file_path.to_string(), FileIndex { hunks, all_hunks });
        }

        Ok(Self { files })
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
        if let Some((file_path, suffix)) = hunk_id.rsplit_once('#') {
            if let Some(h_num) = suffix.strip_prefix('H') {
                if let Ok(hunk_idx) = h_num.parse::<usize>() {
                    if hunk_idx > 0 {
                        return Some((file_path.to_string(), hunk_idx));
                    }
                }
            }
        }
        None
    }

    fn walk_hunk_lines<F>(hunk: &unidiff::Hunk, coords: (u32, u32), mut f: F)
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
    pub fn generate_unified_manifest(&self) -> String {
        let mut result = String::new();

        result.push_str("# Unified Diff Manifest (Copy-Paste Ready)\n\n");
        result.push_str("## How to Use\n\n");
        result.push_str("**For tasks:** Use `hunk_ids` with the IDs below.\n");
        result.push_str("**For feedback:** Use `hunk_id` + copy the exact line content.\n\n");

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

                result.push_str(&format!("### {}\n", hunk_id));
                result.push_str(&format!(
                    "- Old lines: {}-{} ({})\n",
                    coords.0, old_end, hunk.source_length
                ));
                result.push_str(&format!(
                    "- New lines: {}-{} ({})\n",
                    coords.1, new_end, hunk.target_length
                ));
                result.push_str("```diff\n");

                for line in hunk.lines().iter() {
                    let display_line = line.value.trim_end();
                    let prefix = match line.line_type.as_str() {
                        unidiff::LINE_TYPE_ADDED => "+",
                        unidiff::LINE_TYPE_REMOVED => "-",
                        _ => " ",
                    };
                    result.push_str(&format!("{}{}\n", prefix, display_line));
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
    pub fn get_hunk_ranges(&self, file_path: &str) -> (Vec<(u32, u32)>, Vec<(u32, u32)>) {
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
        assert!(manifest.contains("### src/main.rs#H1"));
        assert!(manifest.contains("Old lines:"));
        assert!(manifest.contains("New lines:"));
        assert!(manifest.contains("```diff"));
        assert!(manifest.contains("+"));
        assert!(manifest.contains("-"));
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
}
