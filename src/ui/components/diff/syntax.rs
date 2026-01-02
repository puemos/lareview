use eframe::egui;
use lru::LruCache;
use once_cell::sync::Lazy;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use two_face::syntax;

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(syntax::extra_newlines);
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

const MAX_LINE_LENGTH: usize = 2000;

#[derive(Clone)]
pub struct SyntaxHighlightCache(Arc<Mutex<LruCache<String, Arc<[SyntaxToken]>>>>);

impl Default for SyntaxHighlightCache {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(LruCache::new(
            NonZeroUsize::new(300).unwrap(),
        ))))
    }
}

impl SyntaxHighlightCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &str) -> Option<Arc<[SyntaxToken]>> {
        self.0.lock().unwrap().get(key).cloned()
    }

    pub fn insert(&self, key: String, tokens: Arc<[SyntaxToken]>) {
        self.0.lock().unwrap().put(key, tokens);
    }
}

#[derive(Debug, Clone)]
pub struct SyntaxToken {
    pub color: egui::Color32,
    pub text: String,
}

pub fn detect_language(file_path: &str) -> Option<String> {
    let path = std::path::Path::new(file_path);
    let ext = path.extension().and_then(|e| e.to_str())?;
    SYNTAX_SET
        .find_syntax_by_extension(ext)
        .map(|s| s.name.to_string())
}

pub fn highlight_line(
    content: &str,
    language: &str,
    fallback_color: egui::Color32,
) -> Arc<[SyntaxToken]> {
    if content.len() > MAX_LINE_LENGTH {
        return Arc::new([SyntaxToken {
            color: fallback_color,
            text: content.to_string(),
        }]);
    }

    let syntax = SYNTAX_SET
        .find_syntax_by_token(language)
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    let theme = &THEME_SET.themes["base16-ocean.dark"];
    let mut h = HighlightLines::new(syntax, theme);

    let mut tokens = Vec::new();
    let line = format!("{}\n", content.trim_end_matches('\n'));

    match h.highlight_line(&line, &SYNTAX_SET) {
        Ok(ranges) => {
            for (style, text) in ranges {
                let text = text.trim_end_matches('\n').to_string();
                if !text.is_empty() {
                    tokens.push(SyntaxToken {
                        color: egui::Color32::from_rgb(
                            style.foreground.r,
                            style.foreground.g,
                            style.foreground.b,
                        ),
                        text,
                    });
                }
            }
        }
        Err(_) => {
            tokens.push(SyntaxToken {
                color: fallback_color,
                text: content.to_string(),
            });
        }
    }

    if tokens.is_empty() {
        tokens.push(SyntaxToken {
            color: fallback_color,
            text: content.to_string(),
        });
    }

    Arc::from(tokens.into_boxed_slice())
}

pub fn highlight_line_with_cache(
    content: &str,
    language: &str,
    cache: &SyntaxHighlightCache,
    theme: egui::Color32,
) -> Arc<[SyntaxToken]> {
    let key = format!("{:x}:{}", egui::util::hash(content.as_bytes()), language);

    if let Some(cached) = cache.get(&key) {
        return cached;
    }

    let tokens = highlight_line(content, language, theme);
    cache.insert(key, tokens.clone());
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language_rust() {
        assert!(detect_language("src/main.rs").is_some());
    }

    #[test]
    fn test_detect_language_tsx() {
        assert!(detect_language("component.tsx").is_some());
    }

    #[test]
    fn test_highlight_line_rust() {
        let tokens = highlight_line("fn main() {}", "Rust", egui::Color32::WHITE);
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_highlight_line_very_long() {
        let long = "x".repeat(3000);
        let tokens = highlight_line(&long, "Rust", egui::Color32::WHITE);
        assert_eq!(tokens.len(), 1);
    }
}
