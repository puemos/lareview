use eframe::egui;

/// Returns a RichText configured with the GeistBold font family.
pub fn bold(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).family(egui::FontFamily::Name("GeistBold".into()))
}

/// Returns a RichText configured with the Geist font family (Regular).
pub fn body(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).family(egui::FontFamily::Name("Geist".into()))
}

/// Large bold heading
pub fn h1(text: impl Into<String>) -> egui::RichText {
    bold(text).size(20.0)
}

/// Medium bold heading
pub fn h2(text: impl Into<String>) -> egui::RichText {
    bold(text).size(16.0)
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

/// Returns a RichText configured with the GeistMono font family.
pub fn mono(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).family(egui::FontFamily::Name("GeistMono".into()))
}

/// Small monospace text
pub fn small_mono(text: impl Into<String>) -> egui::RichText {
    mono(text).size(10.0)
}

// --- FontId Helpers ---

pub fn bold_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("GeistBold".into()))
}

pub fn body_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("Geist".into()))
}

pub fn mono_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("GeistMono".into()))
}
