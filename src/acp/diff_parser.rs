#![allow(dead_code)]
//! Diff parser - parse unified git diff into file hunks

use crate::domain::ParsedFileDiff;

/// Parse a unified git diff into individual file diffs
pub fn parse_diff(diff_text: &str) -> Vec<ParsedFileDiff> {
    let lines: Vec<&str> = diff_text.lines().collect();
    let mut results: Vec<ParsedFileDiff> = Vec::new();
    let mut current: Option<ParsedFileDiff> = None;
    let mut buffer: Vec<String> = Vec::new();

    let finalize = |current: &mut Option<ParsedFileDiff>,
                    buffer: &mut Vec<String>,
                    results: &mut Vec<ParsedFileDiff>| {
        if let Some(mut diff) = current.take() {
            diff.patch = buffer.join("\n");
            results.push(diff);
        }
        buffer.clear();
    };

    for line in lines {
        if line.starts_with("diff --git ") {
            finalize(&mut current, &mut buffer, &mut results);

            // Parse file path from diff header
            // Format: diff --git a/path/to/file b/path/to/file
            let file_path = if let Some(rest) = line.strip_prefix("diff --git ") {
                rest.split_whitespace()
                    .last()
                    .map(|s| s.strip_prefix("b/").unwrap_or(s))
                    .unwrap_or("unknown")
                    .to_string()
            } else {
                "unknown".to_string()
            };

            current = Some(ParsedFileDiff {
                file_path,
                patch: String::new(),
                additions: 0,
                deletions: 0,
            });
            buffer.push(line.to_string());
            continue;
        }

        if current.is_none() {
            continue;
        }

        buffer.push(line.to_string());

        // Skip diff metadata lines
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }

        // Count additions and deletions
        if let Some(ref mut diff) = current {
            if line.starts_with('+') && !line.starts_with("+++") {
                diff.additions += 1;
            } else if line.starts_with('-') && !line.starts_with("---") {
                diff.deletions += 1;
            }
        }
    }

    finalize(&mut current, &mut buffer, &mut results);
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_diff() {
        let diff = r#"diff --git a/src/main.rs b/src/main.rs
index abc123..def456 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
-    println!("Hello");
+    println!("Hello, World!");
+    println!("Goodbye!");
 }
"#;

        let files = parse_diff(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_path, "src/main.rs");
        assert_eq!(files[0].additions, 2);
        assert_eq!(files[0].deletions, 1);
    }

    #[test]
    fn test_parse_multiple_files() {
        let diff = r#"diff --git a/file1.rs b/file1.rs
--- a/file1.rs
+++ b/file1.rs
@@ -1 +1 @@
-old
+new
diff --git a/file2.rs b/file2.rs
--- a/file2.rs
+++ b/file2.rs
@@ -1 +1,2 @@
 existing
+added
"#;

        let files = parse_diff(diff);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].file_path, "file1.rs");
        assert_eq!(files[1].file_path, "file2.rs");
    }
}
