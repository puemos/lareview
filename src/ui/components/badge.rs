use eframe::egui;

use crate::ui::spacing;
use crate::ui::theme;

pub fn badge(ui: &mut egui::Ui, text: &str, bg: egui::Color32, fg: egui::Color32) {
    let theme = theme::current_theme();
    egui::Frame::NONE
        .fill(bg)
        .stroke(egui::Stroke::new(1.0, theme.border))
        .corner_radius(egui::CornerRadius::same(255))
        .inner_margin(egui::Margin::symmetric(
            spacing::SPACING_SM as i8,
            spacing::SPACING_XS as i8,
        ))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).size(10.0).color(fg));
        });
}
