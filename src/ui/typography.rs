use eframe::egui;

use crate::ui::theme;

pub(crate) fn geist_bold() -> egui::FontFamily {
    #[cfg(test)]
    {
        egui::FontFamily::Proportional
    }
    #[cfg(not(test))]
    {
        egui::FontFamily::Name("GeistBold".into())
    }
}

pub(crate) fn geist_italic() -> egui::FontFamily {
    #[cfg(test)]
    {
        egui::FontFamily::Proportional
    }
    #[cfg(not(test))]
    {
        egui::FontFamily::Name("GeistItalic".into())
    }
}

/// Returns a RichText configured with the Proportional font family and strong weight.
pub fn bold(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).family(geist_bold()).strong()
}

/// Returns a RichText configured with the Proportional font family.
pub fn body(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).family(egui::FontFamily::Proportional)
}

/// Large bold heading
pub fn h1(text: impl Into<String>) -> egui::RichText {
    let theme = theme::current_theme();
    body(text).size(20.0).color(theme.text_primary).strong()
}

/// Medium bold heading
pub fn h2(text: impl Into<String>) -> egui::RichText {
    let theme = theme::current_theme();
    body(text).size(16.0).color(theme.text_primary).strong()
}

/// Standard UI label size (small)
pub fn label(text: impl Into<String>) -> egui::RichText {
    body(text).size(13.0)
}

/// Bold UI label size (13.0)
pub fn bold_label(text: impl Into<String>) -> egui::RichText {
    bold(text).size(13.0)
}

/// Small text
pub fn small(text: impl Into<String>) -> egui::RichText {
    body(text).size(11.0)
}

/// Extra small text
pub fn tiny(text: impl Into<String>) -> egui::RichText {
    body(text).size(10.0)
}

/// Muted body text
pub fn weak(text: impl Into<String>) -> egui::RichText {
    body(text).weak()
}

/// Returns a RichText configured with the Monospace font family.
pub fn mono(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).family(egui::FontFamily::Monospace)
}

/// Small monospace text
pub fn small_mono(text: impl Into<String>) -> egui::RichText {
    mono(text).size(10.0)
}

// --- FontId Helpers ---

pub fn bold_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, geist_bold())
}

pub fn body_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Proportional)
}

pub fn mono_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Monospace)
}
