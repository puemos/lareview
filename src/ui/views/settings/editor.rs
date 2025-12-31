use crate::ui::app::{Action, LaReviewApp, SettingsAction};
use crate::ui::theme;
use crate::ui::{spacing, typography};
use eframe::egui;

impl LaReviewApp {
    pub fn ui_settings_editor(&mut self, ui: &mut egui::Ui) {
        let theme = theme::current_theme();
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(
                spacing::SPACING_XL as i8,
                spacing::SPACING_MD as i8,
            ))
            .show(ui, |ui| {
                let editors = crate::infra::editor::list_available_editors();
                let preferred_id = self.state.ui.preferred_editor_id.clone();
                let selected_value = preferred_id.as_deref().unwrap_or("none");

                let mut options = Vec::with_capacity(editors.len() + 1);
                options.push(crate::ui::components::PopupOption {
                    label: "None",
                    value: "none",
                    fg: theme.text_disabled,
                    icon: None,
                });
                for editor in &editors {
                    options.push(crate::ui::components::PopupOption {
                        label: editor.label,
                        value: editor.id,
                        fg: theme.text_primary,
                        icon: None,
                    });
                }

                ui.horizontal(|ui| {
                    ui.label(typography::label("Default Editor").color(theme.text_primary));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let status_label = if preferred_id.is_some() {
                            "Selected"
                        } else {
                            "Not Set"
                        };
                        let status_color = if preferred_id.is_some() {
                            theme.success
                        } else {
                            theme.text_disabled
                        };
                        ui.label(typography::label(status_label).color(status_color));
                    });
                });

                ui.add_space(spacing::SPACING_SM);

                if let Some(next) = crate::ui::components::popup_selector(
                    ui,
                    ui.make_persistent_id("default_editor_selector"),
                    selected_value,
                    &options,
                    240.0,
                    true,
                ) {
                    if next == "none" {
                        self.dispatch(Action::Settings(SettingsAction::ClearPreferredEditor));
                    } else {
                        self.dispatch(Action::Settings(SettingsAction::SetPreferredEditor(
                            next.to_string(),
                        )));
                    }
                }

                if editors.is_empty() {
                    ui.add_space(spacing::SPACING_SM);
                    ui.label(
                        typography::weak("No supported editors detected on PATH.")
                            .color(theme.warning),
                    );
                }
            });
    }
}
