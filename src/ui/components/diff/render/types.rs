use super::super::model::ChangeType;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum RowType {
    FileHeader { file_idx: usize },
    DiffLine { file_idx: usize, line_idx: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LineCacheKey {
    pub file_idx: usize,
    pub line_idx: u32,
}

#[derive(Debug, Clone)]
pub struct CachedLineInfo {
    pub old_line_num: Option<usize>,
    pub new_line_num: Option<usize>,
    pub content: String,
    pub change_type: ChangeType,
    pub inline_segments: Option<Vec<(String, bool)>>,
}

#[derive(Debug, Clone)]
pub struct LineCache {
    pub cache: HashMap<LineCacheKey, CachedLineInfo>,
    pub max_size: usize,
}

impl LineCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
        }
    }

    pub fn get(&self, key: &LineCacheKey) -> Option<&CachedLineInfo> {
        self.cache.get(key)
    }

    pub fn insert(&mut self, key: LineCacheKey, value: CachedLineInfo) {
        if self.cache.len() >= self.max_size
            && let Some(first_key) = self.cache.keys().next().cloned()
        {
            self.cache.remove(&first_key);
        }
        self.cache.insert(key, value);
    }
}

pub struct DiffLineInfo {
    pub old_line_num: Option<usize>,
    pub new_line_num: Option<usize>,
    pub content: String,
    pub change_type: ChangeType,
    pub inline_segments: Option<Vec<(String, bool)>>,
}
