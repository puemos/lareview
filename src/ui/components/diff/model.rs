// No imports needed for the new architecture

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

/// Light state stored in egui memory - only keep small UI flags here
#[derive(Default, Clone)]
pub struct DiffViewState {
    pub last_hash: u64,
    pub parse_error: Option<String>,
    pub collapsed: Vec<bool>, // Per-file collapse state - only this is stored in egui memory
}

impl DiffViewState {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }
}
