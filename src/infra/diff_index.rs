//! A module for indexing and querying unified diffs.
//!
//! This module provides a `DiffIndex` that can be created from a unified diff string.
//! The index allows for efficient querying of diff statistics and reconstruction of
//! partial diffs based on `DiffRef` pointers.

use crate::domain::{DiffRef, HunkRef};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    /// A string identifier for the hunk based on its position and content
    hunk_id: String,
}

/// An index for a single file within a larger diff.
#[derive(Debug, Clone)]
struct FileIndex {
    /// A map from (old_start, new_start) to the indexed hunk.
    hunks: HashMap<(u32, u32), IndexedHunk>,
    /// A map from hunk ID to the indexed hunk.
    hunks_by_id: HashMap<String, IndexedHunk>,
    /// A list of all available hunks in order.
    all_hunks: Vec<IndexedHunk>,
}

/// An index for a unified diff, allowing for efficient queries.
#[derive(Debug, Clone)]
pub struct DiffIndex {
    files: HashMap<String, FileIndex>,
}

impl DiffIndex {
    /// Returns all file paths in the diff index
    pub fn get_all_file_paths(&self) -> Vec<&String> {
        self.files.keys().collect()
    }
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
            let file_path = file
                .target_file
                .strip_prefix("b/")
                .unwrap_or(&file.target_file);

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
                    hunk_id: hunk_id.clone(),
                };

                hunks.insert(coords, indexed_hunk.clone());
                hunks_by_id.insert(hunk_id, indexed_hunk.clone());
                all_hunks.push(indexed_hunk);
            }

            files.insert(
                file_path.to_string(),
                FileIndex {
                    hunks,
                    hunks_by_id,
                    all_hunks,
                },
            );
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
                result.push_str(&indexed_hunk.hunk.to_string());
            }
        }

        Ok((result, ordered_files))
    }

    /// Returns all hunks available in the specified file.
    pub fn available_hunks(&self, file_path: &str) -> Vec<HunkRef> {
        if let Some(file_index) = self.files.get(file_path) {
            file_index
                .all_hunks
                .iter()
                .map(|indexed_hunk| HunkRef {
                    old_start: indexed_hunk.coords.0,
                    old_lines: indexed_hunk.hunk.source_length as u32,
                    new_start: indexed_hunk.coords.1,
                    new_lines: indexed_hunk.hunk.target_length as u32,
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Returns hunk IDs and coordinates for the diff manifest tool
    pub fn get_file_hunk_entries(&self, file_path: &str) -> Vec<(String, HunkRef)> {
        if let Some(file_index) = self.files.get(file_path) {
            file_index
                .all_hunks
                .iter()
                .map(|indexed_hunk| {
                    let hunk_ref = HunkRef {
                        old_start: indexed_hunk.coords.0,
                        old_lines: indexed_hunk.hunk.source_length as u32,
                        new_start: indexed_hunk.coords.1,
                        new_lines: indexed_hunk.hunk.target_length as u32,
                    };
                    (indexed_hunk.hunk_id.clone(), hunk_ref)
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Resolves a hunk ID to its HunkRef coordinates.
    pub fn resolve_hunk_id(
        &self,
        file_path: &str,
        hunk_id: &str,
    ) -> Result<HunkRef, DiffIndexError> {
        // The hunk ID is in format "file_path#Hn" where n is the hunk number (1-indexed)
        // We need to extract the number and find the n-th hunk in the file
        if let Some(index_str) = hunk_id.strip_prefix(&format!(
            "{}#H",
            file_path.replace("/", "_").replace("-", "_")
        )) && let Ok(index) = index_str.parse::<usize>()
            && index > 0
        {
            // Get the file index
            let file_index =
                self.files
                    .get(file_path)
                    .ok_or_else(|| DiffIndexError::FileNotFound {
                        file: file_path.to_string(),
                    })?;

            // Get the hunk at the specified index (1-indexed)
            if index <= file_index.all_hunks.len() {
                let indexed_hunk = &file_index.all_hunks[index - 1]; // Convert to 0-indexed
                return Ok(HunkRef {
                    old_start: indexed_hunk.coords.0,
                    old_lines: indexed_hunk.hunk.source_length as u32,
                    new_start: indexed_hunk.coords.1,
                    new_lines: indexed_hunk.hunk.target_length as u32,
                });
            }
        }

        // If the ID doesn't match the expected format or the hunk doesn't exist,
        // try direct lookup in the hunk map (for backward compatibility)
        let file_index = self
            .files
            .get(file_path)
            .ok_or_else(|| DiffIndexError::FileNotFound {
                file: file_path.to_string(),
            })?;

        let indexed_hunk =
            file_index
                .hunks_by_id
                .get(hunk_id)
                .ok_or_else(|| DiffIndexError::InvalidHunkId {
                    file: file_path.to_string(),
                    hunk_id: hunk_id.to_string(),
                })?;

        Ok(HunkRef {
            old_start: indexed_hunk.coords.0,
            old_lines: indexed_hunk.hunk.source_length as u32,
            new_start: indexed_hunk.coords.1,
            new_lines: indexed_hunk.hunk.target_length as u32,
        })
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
---	a/src/main.rs
+++	b/src/main.rs
@@ -1,5 +1,5 @@
fn main() {
-    println!("Hello, world!");
+    println!("Hello, Gemini!");
    println!("Another line");
}

diff --git a/src/lib.rs b/src/lib.rs
new file mode 100644
index 0000000..abcdefg
---	/dev/null
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
---	a/file.txt
+++	b/file.txt
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
}
