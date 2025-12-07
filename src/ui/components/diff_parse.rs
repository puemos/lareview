//! Unified diff parsing extracted from old gpui code
//! Keep the logic, remove gpui UI parts.

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub file_path: String,
    pub patch: Vec<String>,
}

// copy your original parse_diff logic here exactly
pub fn parse_diff(diff_text: &str) -> Vec<FileDiff> {
    let mut results = Vec::new();
    let mut current_file = None;
    let mut current_patch = Vec::new();

    for line in diff_text.lines() {
        if line.starts_with("diff --git") {
            if let Some(file) = current_file.take() {
                results.push(FileDiff {
                    file_path: file,
                    patch: current_patch.clone(),
                });
                current_patch.clear();
            }
            current_file = Some(extract_file_path(line));
        }
        current_patch.push(line.to_string());
    }

    if let Some(file) = current_file {
        results.push(FileDiff {
            file_path: file,
            patch: current_patch,
        });
    }

    results
}

fn extract_file_path(line: &str) -> String {
    // same logic you already had
    line.split_whitespace()
        .last()
        .unwrap_or("unknown")
        .to_string()
}
