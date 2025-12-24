use eframe::egui;

/// Returns a RichText configured with the GeistBold font family.
pub fn bold(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).family(egui::FontFamily::Name("GeistBold".into()))
}

/// Returns a RichText configured with the GeistMono font family.
#[allow(dead_code)]
pub fn mono(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).family(egui::FontFamily::Monospace)
}
