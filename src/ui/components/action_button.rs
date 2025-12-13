use eframe::egui;

use crate::ui::theme;

/// Common action button component
pub fn action_button(
    ui: &mut egui::Ui,
    label: &str,
    enabled: bool,
    color_if_enabled: egui::Color32,
) -> egui::Response {
    let theme = theme::current_theme();
    let button_text = egui::RichText::new(label).size(15.0).color(if enabled {
        theme.text_inverse
    } else {
        theme.text_disabled
    });

    let fill_color = if enabled {
        color_if_enabled
    } else {
        theme.bg_card
    };
    let stroke_color = if enabled {
        theme.text_disabled
    } else {
        theme.border
    };

    let button = egui::Button::new(button_text)
        .fill(fill_color)
        .stroke(egui::Stroke::new(1.0, stroke_color))
        .min_size(egui::vec2(140.0, 32.0));

    let resp = ui.add_enabled(enabled, button);

    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}
