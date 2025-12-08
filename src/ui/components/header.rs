use catppuccin_egui::MOCHA;
use eframe::egui;

pub fn header(ui: &mut egui::Ui, title: &str, action: Option<HeaderAction<'_>>) {
    ui.horizontal(|ui| {
        ui.heading(egui::RichText::new(title).size(20.0).color(MOCHA.text));

        if let Some(action) = action {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if action_button(ui, action.label, action.enabled, action.color_if_enabled)
                    .clicked()
                {
                    (action.on_click)();
                }
            });
        }
    });
}

pub struct HeaderAction<'a> {
    pub label: &'a str,
    pub enabled: bool,
    pub color_if_enabled: egui::Color32,
    pub on_click: Box<dyn FnOnce() + 'a>,
}

impl<'a> HeaderAction<'a> {
    pub fn new(
        label: &'a str,
        enabled: bool,
        color_if_enabled: egui::Color32,
        on_click: impl FnOnce() + 'a,
    ) -> Self {
        Self {
            label,
            enabled,
            color_if_enabled,
            on_click: Box::new(on_click),
        }
    }
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
