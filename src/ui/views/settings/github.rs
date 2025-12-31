use crate::ui::app::{Action, LaReviewApp, SettingsAction};
use crate::ui::theme;
use crate::ui::{icons, spacing, typography};
use eframe::egui;

impl LaReviewApp {
    pub fn ui_settings_github(&mut self, ui: &mut egui::Ui) {
        let theme = theme::current_theme();

        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(
                spacing::SPACING_XL as i8,
                spacing::SPACING_MD as i8,
            ))
            .show(ui, |ui| {
                // Title Block
                ui.horizontal(|ui| {
                    ui.label(typography::label("GitHub CLI Integration").color(theme.text_primary));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.state.session.is_gh_status_checking {
                            ui.label(typography::body("Checking...").color(theme.warning));
                        } else if self.state.session.gh_status.is_some()
                            && self.state.session.gh_status_error.is_none()
                        {
                            ui.label(
                                typography::label(format!("{} Ready", icons::ICON_CHECK))
                                    .color(theme.success),
                            );
                        } else {
                            ui.label(
                                typography::label(format!(
                                    "{} Error/Unknown",
                                    icons::STATUS_IGNORED
                                ))
                                .color(theme.destructive),
                            );
                        }
                    });
                });

                ui.add_space(spacing::SPACING_SM);

                // Content Block
                egui::Grid::new("gh_settings_grid")
                    .num_columns(2)
                    .spacing([spacing::SPACING_LG, spacing::SPACING_MD])
                    .show(ui, |ui| {
                        ui.label(typography::label("Connection:"));
                        ui.horizontal(|ui| {
                            if let Some(err) = &self.state.session.gh_status_error {
                                ui.label(
                                    typography::label("Disconnected").color(theme.destructive),
                                );
                                ui.label(typography::weak(format!("(Error: {})", err)));
                            } else if let Some(status) = &self.state.session.gh_status {
                                ui.label(typography::label("Connected").color(theme.success));
                                if let Some(login) = &status.login {
                                    ui.label(
                                        typography::bold(format!("(@{})", login))
                                            .color(theme.text_disabled),
                                    );
                                }
                            } else {
                                ui.label(typography::label("Unknown").color(theme.warning));
                            }
                        });
                        ui.end_row();

                        if let Some(status) = &self.state.session.gh_status {
                            ui.label(typography::label("Executable Path:"));
                            ui.label(typography::mono(&status.gh_path));
                            ui.end_row();
                        }
                    });

                ui.add_space(spacing::SPACING_MD);

                ui.horizontal(|ui| {
                    let btn_label = if self.state.session.is_gh_status_checking {
                        typography::label("Checking...")
                    } else {
                        typography::label("Refresh Status")
                    };
                    if ui
                        .add_enabled(
                            !self.state.session.is_gh_status_checking,
                            egui::Button::new(btn_label),
                        )
                        .clicked()
                    {
                        self.dispatch(Action::Settings(SettingsAction::CheckGitHubStatus));
                    }
                });

                // Troubleshooting
                if self.state.session.gh_status.is_none()
                    || self.state.session.gh_status_error.is_some()
                {
                    ui.add_space(spacing::SPACING_LG);
                    egui::CollapsingHeader::new(
                        typography::bold("Setup Instructions").color(theme.text_secondary),
                    )
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.add_space(spacing::SPACING_SM);
                        egui::Frame::NONE
                            .fill(theme.bg_secondary)
                            .inner_margin(spacing::SPACING_MD)
                            .corner_radius(crate::ui::spacing::RADIUS_MD)
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    self.ui_copyable_command(
                                        ui,
                                        "1. Install via Homebrew",
                                        "brew install gh",
                                    );
                                    ui.add_space(spacing::SPACING_MD);
                                    self.ui_copyable_command(
                                        ui,
                                        "2. Authenticate",
                                        "gh auth login",
                                    );
                                });
                            });
                    });
                }
            });
    }
}
