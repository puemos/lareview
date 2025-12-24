use crate::ui::{spacing, theme, typography};
use eframe::egui;

#[allow(dead_code)]
pub fn task_status_chip(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    selected: bool,
    tint: egui::Color32,
) -> egui::Response {
    let theme = theme::current_theme();
    let (fill, stroke, fg) = if selected {
        (tint.gamma_multiply(0.18), tint, tint)
    } else {
        (theme.bg_secondary, theme.border, theme.text_disabled)
    };

    let text = typography::body(format!("{icon} {label}"))
        .size(10.0)
        .color(fg);

    let old_padding = ui.spacing().button_padding;
    ui.spacing_mut().button_padding =
        egui::vec2(spacing::BUTTON_PADDING.0, spacing::BUTTON_PADDING.1);

    let resp = ui.add(
        egui::Button::new(text)
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, stroke))
            .corner_radius(egui::CornerRadius::same(255))
            .min_size(egui::vec2(0.0, 28.0)),
    );

    ui.spacing_mut().button_padding = old_padding;

    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}
