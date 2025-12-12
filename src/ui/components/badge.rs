use catppuccin_egui::MOCHA;
use eframe::egui;

pub fn badge(ui: &mut egui::Ui, text: &str, bg: egui::Color32, fg: egui::Color32) {
    egui::Frame::NONE
        .fill(bg)
        .stroke(egui::Stroke::new(1.0, MOCHA.surface2))
        .corner_radius(egui::CornerRadius::same(255))
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).size(10.0).color(fg));
        });
}
