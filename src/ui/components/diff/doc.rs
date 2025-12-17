use std::{ops::Range, sync::Arc};

#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub at_line: u32, // diff line index
    pub old_no: u32,
    pub new_no: u32,
}

#[derive(Debug, Clone)]
pub struct HunkIndex {
    pub body_range: Range<u32>,       // lines inside the hunk body
    pub checkpoints: Vec<Checkpoint>, // every N lines
    pub additions: u32,
    pub deletions: u32,
}

#[derive(Debug, Clone)]
pub struct FileIndex {
    pub old_path: String,
    pub new_path: String,

    // Range of diff line indices in this file, in the unified diff text.
    pub line_range: Range<u32>,

    pub additions: u32,
    pub deletions: u32,

    pub hunks: Vec<HunkIndex>,
}

#[derive(Debug, Clone)]
pub struct DiffDoc {
    pub text: Arc<str>,

    // Byte offsets for the start of each line in `text`.
    // Example: line i starts at text[line_starts[i]..]
    pub line_starts: Vec<u32>,

    // Parsed files and their line ranges (in "line index" space).
    pub files: Vec<FileIndex>,
}

impl DiffDoc {
    pub fn new(text: Arc<str>) -> Self {
        let line_starts = Self::build_line_starts(&text);

        Self {
            text,
            line_starts,
            files: Vec::new(),
        }
    }

    fn build_line_starts(text: &str) -> Vec<u32> {
        let mut starts = Vec::new();
        starts.push(0); // First line starts at 0

        for (i, ch) in text.char_indices() {
            if ch == '\n' {
                starts.push(i as u32 + 1);
            }
        }

        starts
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    pub fn line_str(&self, line_idx: u32) -> &str {
        let (start, end) = self.line_range_bytes(line_idx);
        &self.text[start..end]
    }

    fn line_range_bytes(&self, line_idx: u32) -> (usize, usize) {
        let i = line_idx as usize;
        if i >= self.line_starts.len() {
            return (0, 0);
        }

        let start = self.line_starts[i] as usize;
        let end = if i + 1 < self.line_starts.len() {
            (self.line_starts[i + 1] as usize).saturating_sub(1) // exclude '\n'
        } else {
            self.text.len()
        };

        (start, end)
    }
}
