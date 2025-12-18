//! Embedded assets for LaReview
//! This module provides access to assets embedded in the binary at compile time.

/// Get the content of an asset by its path.
/// Returns None if the asset is not found.
pub fn get_content(path: &str) -> Option<&'static [u8]> {
    match path {
        // Icons
        "assets/icons/claude.svg" => Some(include_bytes!("../assets/icons/claude.svg")),
        "assets/icons/codex.svg" => Some(include_bytes!("../assets/icons/codex.svg")),
        "assets/icons/gemini.svg" => Some(include_bytes!("../assets/icons/gemini.svg")),
        "assets/icons/grok.svg" => Some(include_bytes!("../assets/icons/grok.svg")),
        "assets/icons/kimi.svg" => Some(include_bytes!("../assets/icons/kimi.svg")),
        "assets/icons/mistral.svg" => Some(include_bytes!("../assets/icons/mistral.svg")),
        "assets/icons/opencode.svg" => Some(include_bytes!("../assets/icons/opencode.svg")),
        "assets/icons/qwen.svg" => Some(include_bytes!("../assets/icons/qwen.svg")),
        "assets/icons/icon-512.png" => Some(include_bytes!("../assets/icons/icon-512.png")),

        // Fonts
        "assets/fonts/SpaceMono-Regular.ttf" => {
            Some(include_bytes!("../assets/fonts/SpaceMono-Regular.ttf"))
        }
        "assets/fonts/Inter-Regular.ttf" => {
            Some(include_bytes!("../assets/fonts/Inter-Regular.ttf"))
        }

        _ => None,
    }
}
