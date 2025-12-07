//! Convert parsed diff structures into domain `Patch` values.
//
// This is a small bridge between the UI-level diff parsing
// (`parse_diff` in diff_view.rs) and the domain model
// (`Patch` in crate::domain).

use crate::domain::Patch;
use crate::ui::components::diff_parse::parse_diff;

/// Extract a flat list of `Patch` items from a unified diff string.
///
/// Each file in the diff becomes one `Patch` with:
/// - `file`: the path
/// - `hunk`: the full patch text for that file
pub fn extract_patches_from_diff(diff: &str) -> Vec<Patch> {
    let file_diffs = parse_diff(diff);

    file_diffs
        .into_iter()
        .map(|f| Patch {
            file: f.file_path,
            hunk: f.patch.join("\n"),
        })
        .collect()
}
