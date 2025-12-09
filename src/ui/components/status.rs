use catppuccin_egui::MOCHA;
use eframe::egui;

/// Common status label component
#[allow(dead_code)]
pub fn status_label(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    ui.label(egui::RichText::new(text).color(color).size(12.0));
}

/// Common error banner component
pub fn error_banner(ui: &mut egui::Ui, error_message: &str) {
    egui::Frame::new()
        .fill(MOCHA.red.gamma_multiply(0.2))
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("error:").color(MOCHA.red));
                ui.label(egui::RichText::new(error_message).color(MOCHA.text));
            });
        });
}
