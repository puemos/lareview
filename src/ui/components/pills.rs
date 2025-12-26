use crate::ui::{spacing, theme};

pub fn pill_action_button(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    enabled: bool,
    tint: egui::Color32,
) -> egui::Response {
    let theme = theme::current_theme();
    let old_padding = ui.spacing().button_padding;

    ui.scope(|ui| {
        ui.spacing_mut().button_padding =
            egui::vec2(spacing::BUTTON_PADDING.0, spacing::BUTTON_PADDING.1);

        // Set text colors by modifying fg_stroke.color for different states
        if enabled {
            ui.style_mut().visuals.widgets.inactive.fg_stroke.color = theme.text_primary;
            ui.style_mut().visuals.widgets.hovered.fg_stroke.color = tint;
            ui.style_mut().visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, tint);
        } else {
            ui.style_mut().visuals.widgets.inactive.fg_stroke.color = theme.text_disabled;
        }

        let fill = if enabled {
            theme.bg_secondary
        } else {
            theme.bg_surface
        };

        // Don't set explicit color - let widget visuals handle it
        let text = egui::RichText::new(format!("{icon} {label}")).size(12.0);

        let button = egui::Button::new(text)
            .fill(fill)
            .corner_radius(egui::CornerRadius::same(5))
            .min_size(egui::vec2(0.0, 28.0));

        let resp = ui
            .add_enabled(enabled, button)
            .on_hover_cursor(egui::CursorIcon::PointingHand);

        ui.spacing_mut().button_padding = old_padding;
        resp
    })
    .inner
}
