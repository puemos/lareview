use eframe::egui;

use crate::ui::spacing;
use crate::ui::typography;

pub fn badge(
    ui: &mut egui::Ui,
    text: impl Into<egui::WidgetText>,
    bg: egui::Color32,
    fg: egui::Color32,
) -> egui::Response {
    let text = match text.into() {
        egui::WidgetText::RichText(rich) => {
            egui::WidgetText::RichText(typography::body((*rich).text()).size(10.0).color(fg).into())
        }
        other => other,
    };

    egui::Frame::NONE
        .fill(bg)
        .stroke(egui::Stroke::new(1.0, fg.gamma_multiply(0.4)))
        .corner_radius(egui::CornerRadius::same(255))
        .inner_margin(egui::Margin::symmetric(
            spacing::SPACING_SM as i8,
            spacing::SPACING_XS as i8,
        ))
        .show(ui, |ui| {
            ui.label(text);
        })
        .response
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    #[test]
    fn test_badge() {
        let mut harness = Harness::new_ui(|ui| {
            badge(ui, "Beta", egui::Color32::RED, egui::Color32::WHITE);
        });
        harness.run();
        harness.get_by_label("Beta");
    }
}
