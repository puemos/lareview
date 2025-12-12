use catppuccin_egui::MOCHA;
use eframe::egui;

pub fn task_status_chip(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    selected: bool,
    tint: egui::Color32,
) -> egui::Response {
    let (fill, stroke, fg) = if selected {
        (tint.gamma_multiply(0.18), tint, tint)
    } else {
        (MOCHA.surface0, MOCHA.surface2, MOCHA.subtext0)
    };

    let text = egui::RichText::new(format!("{icon} {label}"))
        .size(10.0)
        .color(fg);

    let old_padding = ui.spacing().button_padding;
    ui.spacing_mut().button_padding = egui::vec2(8.0, 4.0);

    let resp = ui.add(
        egui::Button::new(text)
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, stroke))
            .corner_radius(egui::CornerRadius::same(255))
            .min_size(egui::vec2(0.0, 22.0)),
    );

    ui.spacing_mut().button_padding = old_padding;
    resp
}
