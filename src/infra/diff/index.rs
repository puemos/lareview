//! A module for indexing and querying unified diffs.
//!
//! This module provides a `DiffIndex` that can be created from a unified diff string.
//! The index allows for efficient querying of diff statistics and reconstruction of
//! partial diffs based on `DiffRef` pointers.

use crate::domain::{DiffRef, HunkRef};
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
    fn test_diff_index_new() {
        let index = DiffIndex::new(TEST_DIFF).unwrap();
        assert_eq!(index.files.len(), 2);
        assert!(index.files.contains_key("src/main.rs"));
        assert!(index.files.contains_key("src/lib.rs"));
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
    fn test_generate_hunk_manifest() {
        let index = DiffIndex::new(TEST_DIFF).unwrap();
        let manifest = index.generate_hunk_manifest();
        assert!(manifest.contains("src/main.rs:"));
        assert!(manifest.contains("H1:"));
    }

    #[test]
    fn test_diff_index_error_display() {
        let err = DiffIndexError::FileNotFound {
            file: "missing.rs".into(),
        };
        assert!(err.to_string().contains("missing.rs"));
        assert!(err.to_json().contains("FileNotFound"));
    }
}
