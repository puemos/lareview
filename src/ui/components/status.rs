use eframe::egui;

use crate::ui::spacing;
use crate::ui::theme;

/// Common status label component
#[allow(dead_code)]
pub fn status_label(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    ui.label(egui::RichText::new(text).color(color).size(12.0));
}

/// Common error banner component
pub fn error_banner(ui: &mut egui::Ui, error_message: &str) {
    if ui.available_width() < 50.0 {
        return;
    }
    let theme = theme::current_theme();
    egui::Frame::new()
        .fill(theme.destructive.gamma_multiply(0.2))
        .inner_margin(egui::Margin::symmetric(
            spacing::SPACING_MD as i8,
            spacing::SPACING_SM as i8,
        ))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("error:").color(theme.destructive));
                ui.label(egui::RichText::new(error_message).color(theme.text_primary));
            });
        });
}
