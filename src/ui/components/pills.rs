use eframe::egui;

use crate::ui::spacing;
use crate::ui::theme;

pub fn pill_divider(ui: &mut egui::Ui) {
    let theme = theme::current_theme();

    // Small, subtle divider to separate inline "pill" elements without adding visual weight.
    let size = egui::vec2(spacing::SPACING_SM, 18.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());

    let x = rect.center().x;
    let top = rect.top() + 2.0;
    let bottom = rect.bottom() - 2.0;

    ui.painter().line_segment(
        [egui::pos2(x, top), egui::pos2(x, bottom)],
        egui::Stroke::new(1.0, theme.border_secondary.gamma_multiply(0.8)),
    );
}

pub fn pill_action_button(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    enabled: bool,
    tint: egui::Color32,
) -> egui::Response {
    let theme = theme::current_theme();
    let text = egui::RichText::new(format!("{icon} {label}"))
        .size(12.0)
        .color(if enabled {
            theme.text_primary
        } else {
            theme.text_disabled
        });

    let fill = if enabled {
        theme.bg_secondary
    } else {
        theme.bg_surface
    };

    let stroke = if enabled { tint } else { theme.border };

    let old_padding = ui.spacing().button_padding;
    ui.spacing_mut().button_padding =
        egui::vec2(spacing::BUTTON_PADDING.0, spacing::BUTTON_PADDING.1);

    let resp = ui.add_enabled(
        enabled,
        egui::Button::new(text)
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, stroke))
            .corner_radius(egui::CornerRadius::same(255))
            .min_size(egui::vec2(0.0, 28.0)),
    );

    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);

    ui.spacing_mut().button_padding = old_padding;
    resp
}
