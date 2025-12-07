use catppuccin_egui::MOCHA;
use eframe::egui;

/// Common header component
pub fn header(ui: &mut egui::Ui, title: &str) {
    ui.heading(egui::RichText::new(title).size(20.0).color(MOCHA.text));
}

/// Header with action button component
pub fn header_with_action(
    ui: &mut egui::Ui,
    title: &str,
    action_label: &str,
    enabled: bool,
    color_if_enabled: egui::Color32,
    on_click: impl FnOnce(),
) {
    ui.horizontal(|ui| {
        header(ui, title);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if action_button(ui, action_label, enabled, color_if_enabled).clicked() {
                on_click();
            }
        });
    });
}

/// Common action button component
pub fn action_button(
    ui: &mut egui::Ui,
    label: &str,
    enabled: bool,
    color_if_enabled: egui::Color32,
) -> egui::Response {
    let button_text = egui::RichText::new(label).size(15.0).color(if enabled {
        MOCHA.crust
    } else {
        MOCHA.subtext0
    });

    let fill_color = if enabled {
        color_if_enabled
    } else {
        MOCHA.surface1
    };
    let stroke_color = if enabled {
        MOCHA.overlay0
    } else {
        MOCHA.surface2
    };

    let button = egui::Button::new(button_text)
        .fill(fill_color)
        .stroke(egui::Stroke::new(1.0, stroke_color))
        .min_size(egui::vec2(140.0, 32.0));

    ui.add_enabled(enabled, button)
}
