use crate::ui::app::{Action, LaReviewApp, SettingsAction};
use crate::ui::theme;
use crate::ui::{icons, spacing, typography};
use eframe::egui;
use which::which;

impl LaReviewApp {
    pub fn ui_settings_cli(&mut self, ui: &mut egui::Ui) {
        let theme = theme::current_theme();

        // Find CLI path dynamically
        let cli_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|parent| parent.join("lareview")))
            .unwrap_or_default();

        let cli_available = which("lareview").is_ok();
        let cli_path_str = cli_path.to_string_lossy().to_string();

        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(
                spacing::SPACING_XL as i8,
                spacing::SPACING_MD as i8,
            ))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(typography::label("CLI Installation").color(theme.text_primary));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.state.ui.is_cli_installing {
                            ui.label(typography::body("Installing...").color(theme.warning));
                        } else if cli_available {
                            ui.label(
                                typography::label(format!("{} Available", icons::ICON_CHECK))
                                    .color(theme.success),
                            );
                        } else {
                            ui.label(
                                typography::label(format!("{} Not on PATH", icons::STATUS_IGNORED))
                                    .color(theme.warning),
                            );
                        }
                    });
                });

                ui.add_space(spacing::SPACING_SM);

                egui::Grid::new("cli_settings_grid")
                    .num_columns(2)
                    .spacing([spacing::SPACING_LG, spacing::SPACING_MD])
                    .show(ui, |ui| {
                        ui.label(typography::label("Status:"));
                        ui.horizontal(|ui| {
                            if cli_available {
                                ui.label(typography::label("Installed").color(theme.success));
                            } else {
                                ui.label(typography::label("Not installed").color(theme.warning));
                            }
                        });
                        ui.end_row();

                        ui.label(typography::label("Binary:"));
                        ui.vertical(|ui| {
                            if !cli_path_str.is_empty() {
                                ui.label(typography::mono(&cli_path_str).color(theme.text_muted));
                            }
                            ui.label(typography::mono("lareview").color(theme.text_muted));
                            ui.label(
                                typography::weak("Run from terminal to open GUI")
                                    .color(theme.text_disabled),
                            );
                        });
                        ui.end_row();
                    });

                ui.add_space(spacing::SPACING_MD);

                if cli_available {
                    ui.label(
                        typography::body("CLI is ready to use from terminal!").color(theme.success),
                    );
                    ui.add_space(spacing::SPACING_MD);
                    ui.horizontal(|ui| {
                        ui.hyperlink_to(
                            "CLI Documentation →",
                            "https://github.com/puemos/lareview#terminal-workflow",
                        );
                    });
                } else if self.state.ui.is_cli_installing {
                    ui.horizontal(|ui| {
                        crate::ui::animations::cyber::cyber_spinner(
                            ui,
                            theme.brand,
                            Some(crate::ui::animations::cyber::CyberSpinnerSize::Sm),
                        );
                        ui.label(typography::body("Configuring PATH..."));
                    });
                } else {
                    let btn = egui::Button::new(typography::label("Add to PATH"));
                    if ui
                        .add_enabled(!self.state.ui.is_cli_installing, btn)
                        .clicked()
                    {
                        self.dispatch(Action::Settings(SettingsAction::InstallCli));
                    }
                }

                if !self.state.ui.cli_install_output.is_empty() {
                    ui.add_space(spacing::SPACING_MD);
                    egui::CollapsingHeader::new(typography::bold("Installation Output"))
                        .default_open(true)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .max_height(150.0)
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(
                                            &mut self.state.ui.cli_install_output.as_str(),
                                        )
                                        .font(typography::mono_font(13.0))
                                        .desired_width(f32::INFINITY)
                                        .lock_focus(true),
                                    );
                                });
                        });
                }

                if self.state.ui.cli_install_success {
                    ui.add_space(spacing::SPACING_MD);
                    ui.label(
                        typography::body("✓ CLI installed. Open a new terminal to use `lareview`.")
                            .color(theme.success),
                    );
                }

                ui.add_space(spacing::SPACING_LG);

                egui::CollapsingHeader::new(
                    typography::bold("Usage Examples").color(theme.text_secondary),
                )
                .default_open(false)
                .show(ui, |ui| {
                    ui.add_space(spacing::SPACING_SM);
                    egui::Frame::NONE
                        .fill(theme.bg_secondary)
                        .inner_margin(spacing::SPACING_MD)
                        .corner_radius(crate::ui::spacing::RADIUS_MD)
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                self.ui_copyable_command(ui, "Open with current repo", "lareview");
                                ui.add_space(spacing::SPACING_SM);
                                self.ui_copyable_command(
                                    ui,
                                    "Compare branches",
                                    "lareview main feature",
                                );
                                ui.add_space(spacing::SPACING_SM);
                                self.ui_copyable_command(
                                    ui,
                                    "Pipe diff to GUI",
                                    "git diff HEAD | lareview",
                                );
                                ui.add_space(spacing::SPACING_SM);
                                self.ui_copyable_command(
                                    ui,
                                    "Review a PR",
                                    "lareview pr owner/repo#123",
                                );
                            });
                        });
                });
            });
    }
}
