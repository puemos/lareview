use crate::domain::ReviewSource;
use crate::infra::diff::index::DiffIndex;
use serde::{Deserialize, Serialize};

#[tauri::command]
pub fn parse_diff(diff_text: String) -> Result<ParsedDiff, String> {
    let index = DiffIndex::new(&diff_text).map_err(|e| e.to_string())?;
    let manifest = index.generate_hunk_manifest_json();
    let file_paths = index.get_all_file_paths();

    let total_additions = diff_text
        .lines()
        .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
        .count();

    let total_deletions = diff_text
        .lines()
        .filter(|l| l.starts_with('-') && !l.starts_with("---"))
        .count();

    let files: Vec<ParsedDiffFile> = file_paths
        .iter()
        .map(|path| {
            let hunk_ids = index.get_hunk_ids_for_file(path);
            ParsedDiffFile {
                name: path.clone(),
                old_path: path.clone(),
                new_path: path.clone(),
                hunks: hunk_ids
                    .iter()
                    .filter_map(|hunk_id| {
                        index.get_hunk_coords(hunk_id).map(|coords| {
                            let content = index
                                .get_hunk_content_by_coords(
                                    path,
                                    coords.old_start,
                                    coords.new_start,
                                )
                                .unwrap_or_default();

                            ParsedHunk {
                                old_start: coords.old_start,
                                old_lines: coords.old_lines,
                                new_start: coords.new_start,
                                new_lines: coords.new_lines,
                                content,
                            }
                        })
                    })
                    .collect(),
            }
        })
        .collect();

    Ok(ParsedDiff {
        diff_text,
        total_additions,
        total_deletions,
        hunk_manifest: manifest,
        files,
        source: None,
        title: None,
    })
}

#[tauri::command]
pub fn get_file_content(
    repo_root: String,
    file_path: String,
    commit: String,
) -> Result<String, String> {
    use std::process::Command;
    let output = Command::new("git")
        .args(["show", &format!("{}:{}", commit, file_path)])
        .current_dir(&repo_root)
        .output()
        .map_err(|e| e.to_string())?;

    String::from_utf8(output.stdout).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDiff {
    pub diff_text: String,
    pub total_additions: usize,
    pub total_deletions: usize,
    pub hunk_manifest: String,
    #[serde(default)]
    pub files: Vec<ParsedDiffFile>,
    #[serde(default)]
    pub source: Option<ReviewSource>,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDiffFile {
    pub name: String,
    pub old_path: String,
    pub new_path: String,
    pub hunks: Vec<ParsedHunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    #[serde(default)]
    pub content: String,
}
