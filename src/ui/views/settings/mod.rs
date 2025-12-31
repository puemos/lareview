pub mod agents;
pub mod d2;
pub mod editor;
pub mod github;
pub mod overlays;

use crate::ui::app::LaReviewApp;
use crate::ui::spacing::TOP_HEADER_HEIGHT;
use crate::ui::theme;
use crate::ui::{spacing, typography};
use eframe::egui;

impl LaReviewApp {
    pub fn ui_settings(&mut self, ui: &mut egui::Ui) {
        if ui.available_width() < 100.0 {
            return;
        }

        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(spacing::SPACING_XL as i8, 0))
            .show(ui, |ui| {
                ui.set_min_height(TOP_HEADER_HEIGHT);
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), TOP_HEADER_HEIGHT),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.horizontal(|ui| ui.label(typography::h2("Settings")));
                    },
                );
            });

        ui.separator();

        self.ui_settings_github(ui);
        ui.separator();

        self.ui_settings_d2(ui);
        ui.separator();

        self.ui_settings_editor(ui);
        ui.separator();

        self.ui_settings_agents(ui);
    }

    pub(crate) fn ui_copyable_command(&mut self, ui: &mut egui::Ui, label: &str, cmd: &str) {
        let theme = theme::current_theme();
        ui.vertical(|ui| {
            ui.label(typography::bold(label));
            ui.add_space(spacing::SPACING_XS);
            ui.horizontal(|ui| {
                ui.label(typography::mono(cmd).color(theme.text_secondary));
                if ui.button("Copy").clicked() {
                    self.state.ui.pending_clipboard_copy = Some(cmd.to_owned());
                }
            });
        });
    }
}
