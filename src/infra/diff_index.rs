//! A module for indexing and querying unified diffs.
//!
//! This module provides a `DiffIndex` that can be created from a unified diff string.
//! The index allows for efficient querying of diff statistics and reconstruction of
//! partial diffs based on `DiffRef` pointers.

use crate::domain::{DiffRef, HunkRef};
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use unidiff::{Hunk, PatchSet};

/// An index for a single file within a larger diff.
#[derive(Debug, Clone)]
struct FileIndex {
    /// A map from (old_start, new_start) to the hunk.
    hunks: HashMap<(u32, u32), Hunk>,
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
            return Ok(Self { files: HashMap::new() });
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
            for hunk in file.hunks() {
                // Use the correct fields for the unidiff crate
                hunks.insert(
                    (hunk.source_start as u32, hunk.target_start as u32),
                    hunk.clone(),
                );
            }

            files.insert(
                file_path.to_string(),
                FileIndex {
                    hunks,
                },
            );
        }

        Ok(Self {
            files,
        })
    }

    /// Calculates the total number of additions and deletions for a set of `DiffRef`s.
    pub fn task_stats(&self, diff_refs: &[DiffRef]) -> Result<(u32, u32)> {
        let mut additions = 0;
        let mut deletions = 0;

        for diff_ref in diff_refs {
            let file_index = self
                .files
                .get(&diff_ref.file)
                .ok_or_else(|| anyhow!("File not found in diff index: {}", diff_ref.file))?;

            for hunk_ref in &diff_ref.hunks {
                let hunk = file_index
                    .hunks
                    .get(&(hunk_ref.old_start, hunk_ref.new_start))
                    .ok_or_else(|| {
                        anyhow!(
                            "Hunk not found in file {}: old_start={}, new_start={}",
                            diff_ref.file,
                            hunk_ref.old_start,
                            hunk_ref.new_start
                        )
                    })?;

                for line in hunk.lines() {
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
            .ok_or_else(|| anyhow!("File not found in diff index: {}", file_path))?;

        file_index
            .hunks
            .get(&(hunk_ref.old_start, hunk_ref.new_start))
            .ok_or_else(|| {
                anyhow!(
                    "Hunk not found in file {}: old_start={}, new_start={}",
                    file_path,
                    hunk_ref.old_start,
                    hunk_ref.new_start
                )
            })?;

        Ok(())
    }

    /// Renders a unified diff snippet for the given `DiffRef`s.
    /// Returns the diff string and a list of ordered file paths.
    pub fn render_unified_diff(&self, diff_refs: &[DiffRef]) -> Result<(String, Vec<String>)> {
        let mut result = String::new();
        let mut ordered_files = Vec::new();

        for diff_ref in diff_refs {
            let file_index = self
                .files
                .get(&diff_ref.file)
                .ok_or_else(|| anyhow!("File not found in diff index: {}", diff_ref.file))?;

            // Build the header for this file
            let header = format!(
                "diff --git a/{} b/{}\n--- a/{}\n+++ b/{}\n",
                diff_ref.file, diff_ref.file, diff_ref.file, diff_ref.file
            );
            ordered_files.push(diff_ref.file.clone());
            result.push_str(&header);

            for hunk_ref in &diff_ref.hunks {
                let hunk = file_index
                    .hunks
                    .get(&(hunk_ref.old_start, hunk_ref.new_start))
                    .ok_or_else(|| {
                        anyhow!(
                            "Hunk not found in file {}: old_start={}, new_start={}",
                            diff_ref.file,
                            hunk_ref.old_start,
                            hunk_ref.new_start
                        )
                    })?;
                result.push_str(&hunk.to_string());
            }
        }

        Ok((result, ordered_files))
    }
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
