use crate::ui::app::{Action, LaReviewApp, SettingsAction};
use crate::ui::spacing;
use eframe::egui;

use crate::ui::theme;

impl LaReviewApp {
    pub fn ui_settings(&mut self, ui: &mut egui::Ui) {
        let theme = theme::current_theme();

        // --- GitHub Section ---
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(
                spacing::SPACING_LG as i8,
                spacing::SPACING_MD as i8,
            ))
            .show(ui, |ui| {
                // Title Block
                ui.horizontal(|ui| {
                    ui.strong("GitHub CLI Integration");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.state.is_gh_status_checking {
                            ui.label(egui::RichText::new("Checking...").color(theme.warning));
                        } else if self.state.gh_status.is_some()
                            && self.state.gh_status_error.is_none()
                        {
                            ui.colored_label(theme.success, "✔ Ready");
                        } else {
                            ui.colored_label(theme.destructive, "✖ Error/Unknown");
                        }
                    });
                });

                ui.add_space(spacing::SPACING_SM);

                // Content Block
                egui::Grid::new("gh_settings_grid")
                    .num_columns(2)
                    .spacing([spacing::SPACING_LG, spacing::SPACING_MD])
                    .show(ui, |ui| {
                        ui.label("Connection:");
                        ui.horizontal(|ui| {
                            if let Some(err) = &self.state.gh_status_error {
                                ui.colored_label(theme.destructive, "Disconnected");
                                ui.weak(format!("(Error: {})", err));
                            } else if let Some(status) = &self.state.gh_status {
                                ui.colored_label(theme.success, "Connected");
                                if let Some(login) = &status.login {
                                    ui.label(
                                        egui::RichText::new(format!("(@{})", login))
                                            .color(theme.text_disabled)
                                            .strong(),
                                    );
                                }
                            } else {
                                ui.colored_label(theme.warning, "Unknown");
                            }
                        });
                        ui.end_row();

                        if let Some(status) = &self.state.gh_status {
                            ui.label("Executable Path:");
                            ui.monospace(&status.gh_path);
                            ui.end_row();
                        }
                    });

                ui.add_space(spacing::SPACING_MD);

                ui.horizontal(|ui| {
                    let btn_label = if self.state.is_gh_status_checking {
                        "Checking..."
                    } else {
                        "Refresh Status"
                    };
                    if ui
                        .add_enabled(
                            !self.state.is_gh_status_checking,
                            egui::Button::new(btn_label),
                        )
                        .clicked()
                    {
                        self.dispatch(Action::Settings(SettingsAction::CheckGitHubStatus));
                    }
                });

                // Troubleshooting
                if self.state.gh_status.is_none() || self.state.gh_status_error.is_some() {
                    ui.add_space(spacing::SPACING_LG);
                    egui::CollapsingHeader::new(
                        egui::RichText::new("Setup Instructions")
                            .strong()
                            .color(theme.text_secondary),
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

        ui.separator();

        // --- D2 Section ---
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(
                spacing::SPACING_LG as i8,
                spacing::SPACING_MD as i8,
            ))
            .show(ui, |ui| {
                let d2_installed = crate::infra::brew::find_bin("d2").is_some();

                ui.horizontal(|ui| {
                    ui.strong("D2 Diagram Engine");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.state.is_d2_installing {
                            ui.label(egui::RichText::new("Installing...").color(theme.warning));
                        } else if d2_installed {
                            ui.colored_label(theme.success, "✔ Installed");
                        } else {
                            ui.colored_label(theme.text_disabled, "Not Installed");
                        }
                    });
                });

                ui.add_space(spacing::SPACING_SM);

                let install_cmd = "curl -fsSL https://d2lang.com/install.sh | sh -s --";
                let uninstall_cmd =
                    "curl -fsSL https://d2lang.com/install.sh | sh -s -- --uninstall";

                if d2_installed {
                    ui.label("D2 is ready to render diagrams.");
                    ui.add_space(spacing::SPACING_MD);

                    ui.collapsing("Uninstall Options", |ui| {
                        ui.add_space(spacing::SPACING_SM);
                        self.ui_copyable_command(ui, "Manual Uninstall", uninstall_cmd);

                        ui.add_space(spacing::SPACING_SM);
                        let btn = egui::Button::new("Run Uninstall Script").fill(theme.bg_card);
                        if ui.add_enabled(!self.state.is_d2_installing, btn).clicked() {
                            self.dispatch(Action::Settings(SettingsAction::RequestD2Uninstall));
                        }
                    });
                } else {
                    egui::Frame::NONE
                        .fill(theme.bg_surface)
                        .inner_margin(spacing::SPACING_SM)
                        .stroke(egui::Stroke::new(1.0, theme.warning))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("⚠").color(theme.warning).size(16.0));
                                ui.vertical(|ui| {
                                    ui.strong("Remote Script Warning");
                                    ui.label("Installation requires running a remote shell script. You can run it manually or allow LaReview to run it.");
                                });
                            });
                        });

                    ui.add_space(spacing::SPACING_MD);

                    let mut allow = self.state.allow_d2_install;
                    if ui
                        .checkbox(&mut allow, "I understand and want to proceed")
                        .changed()
                    {
                        self.dispatch(Action::Settings(SettingsAction::SetAllowD2Install(allow)));
                    }

                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        let can_install =
                            self.state.allow_d2_install && !self.state.is_d2_installing;
                        if ui
                            .add_enabled(can_install, egui::Button::new("Install Automatically"))
                            .clicked()
                        {
                            self.dispatch(Action::Settings(SettingsAction::RequestD2Install));
                        }
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    self.ui_copyable_command(ui, "Manual Install Command", install_cmd);
                }

                if self.state.is_d2_installing {
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        crate::ui::animations::cyber::cyber_spinner(ui, theme.brand);
                        ui.label("Processing...");
                    });
                }

                if !self.state.d2_install_output.is_empty() {
                    ui.add_space(12.0);
                    egui::CollapsingHeader::new("Script Output Log")
                        .default_open(true)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .max_height(150.0)
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(
                                            &mut self.state.d2_install_output.as_str(),
                                        )
                                        .font(egui::TextStyle::Monospace)
                                        .desired_width(f32::INFINITY)
                                        .lock_focus(true),
                                    );
                                });
                        });
                }
            });
    }

    /// Helper UI component for commands
    fn ui_copyable_command(&self, ui: &mut egui::Ui, label: &str, cmd: &str) {
        let theme = theme::current_theme();
        ui.label(label);
        ui.horizontal(|ui| {
            // Command text in a box
            egui::Frame::NONE
                .fill(theme.bg_surface)
                .inner_margin(spacing::SPACING_SM) // Using SPACING_SM (8.0) as closest to 6.0
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.monospace(cmd);
                });

            // Copy button
            if ui.button("📋 Copy").clicked() {
                ui.ctx().copy_text(cmd.to_string());
            }
        });
    }
}
