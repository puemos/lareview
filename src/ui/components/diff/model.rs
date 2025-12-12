use std::sync::Arc;

/// Possible actions that can be triggered from the diff viewer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffAction {
    /// No action was triggered.
    None,
    /// Open the diff in full window view.
    OpenFullWindow,
    /// A line was clicked for commenting.
    /// Carries the 0-based index of the line in the FileDiff structure, and the
    /// line number in the source file (old_line_num or new_line_num).
    AddNote {
        file_idx: usize,
        line_idx: usize,
        line_number: usize,
    },
    /// Save a note for a line.
    SaveNote {
        file_idx: usize,
        line_idx: usize,
        line_number: usize,
        note_text: String,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct LineContext {
    pub file_idx: usize,
    pub line_idx: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ChangeType {
    Equal,
    Delete,
    Insert,
}

#[derive(Debug, Clone)]
pub(super) struct DiffLine {
    pub(super) old_line_num: Option<usize>,
    pub(super) new_line_num: Option<usize>,
    pub(super) content: Arc<str>,
    pub(super) change_type: ChangeType,
    pub(super) inline_segments: Option<Vec<(String, bool)>>, // (text, highlight)
}

#[derive(Debug, Clone)]
pub(super) struct FileDiff {
    pub(super) old_path: String,
    pub(super) new_path: String,
    pub(super) lines: Vec<DiffLine>,
    pub(super) additions: usize,
    pub(super) deletions: usize,
}

#[derive(Default, Clone)]
pub(super) struct DiffState {
    pub(super) last_hash: u64,
    pub(super) files: Vec<FileDiff>,
    pub(super) parse_error: Option<String>,
    pub(super) rows: Vec<Row>,
    pub(super) row_height: f32,
    pub(super) collapsed: Vec<bool>, // Per-file collapse state
}

#[derive(Clone)]
pub(super) enum Row {
    FileHeader { file_idx: usize },
    DiffLine { file_idx: usize, line_idx: usize },
}
