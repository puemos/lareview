use catppuccin_egui::MOCHA;
use eframe::egui;

use crate::ui::spacing;

pub fn pill_divider(ui: &mut egui::Ui) {
    ui.add_sized(
        egui::vec2(spacing::SPACING_XS + 2.0, 22.0),
        egui::Separator::default().vertical(),
    );
}

pub fn pill_action_button(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    enabled: bool,
    tint: egui::Color32,
) -> egui::Response {
    let text = egui::RichText::new(format!("{icon} {label}"))
        .size(12.0)
        .color(if enabled { MOCHA.text } else { MOCHA.subtext0 });

    let fill = if enabled {
        MOCHA.surface0
    } else {
        MOCHA.mantle
    };

    let stroke = if enabled { tint } else { MOCHA.surface2 };

    let old_padding = ui.spacing().button_padding;
    ui.spacing_mut().button_padding =
        egui::vec2(spacing::BUTTON_PADDING.0, spacing::BUTTON_PADDING.1);

    let resp = ui.add_enabled(
        enabled,
        egui::Button::new(text)
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, stroke))
            .corner_radius(egui::CornerRadius::same(255))
            .min_size(egui::vec2(0.0, 24.0)),
    );

    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);

    ui.spacing_mut().button_padding = old_padding;
    resp
}
