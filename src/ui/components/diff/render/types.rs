use super::super::model::ChangeType;
use super::super::syntax::SyntaxToken;
use lru::LruCache;
use std::num::NonZeroUsize;

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
    pub syntax_tokens: Option<Vec<SyntaxToken>>,
}

#[derive(Debug, Clone)]
pub struct LineCache(LruCache<LineCacheKey, CachedLineInfo>);

impl LineCache {
    pub fn new(max_size: usize) -> Self {
        Self(LruCache::new(NonZeroUsize::new(max_size).unwrap()))
    }

    pub fn get(&mut self, key: &LineCacheKey) -> Option<&CachedLineInfo> {
        self.0.get(key)
    }

    pub fn insert(&mut self, key: LineCacheKey, value: CachedLineInfo) {
        self.0.put(key, value);
    }
}

pub struct DiffLineInfo {
    pub old_line_num: Option<usize>,
    pub new_line_num: Option<usize>,
    pub content: String,
    pub change_type: ChangeType,
    pub inline_segments: Option<Vec<(String, bool)>>,
    pub syntax_tokens: Option<Vec<SyntaxToken>>,
}
