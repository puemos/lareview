use crate::ui::app::{Action, LaReviewApp, SettingsAction};
use crate::ui::theme;
use crate::ui::{icons, spacing, typography};
use eframe::egui;

impl LaReviewApp {
    pub fn ui_settings_d2(&mut self, ui: &mut egui::Ui) {
        let theme = theme::current_theme();
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(
                spacing::SPACING_XL as i8,
                spacing::SPACING_MD as i8,
            ))
            .show(ui, |ui| {
                let d2_installed = crate::infra::shell::find_bin("d2").is_some();

                ui.horizontal(|ui| {
                    ui.label(typography::label("D2 Diagram Engine").color(theme.text_primary));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.state.ui.is_d2_installing {
                            ui.label(typography::body("Installing...").color(theme.warning));
                        } else if d2_installed {
                            ui.label(
                                typography::label(format!("{} Installed", icons::ICON_CHECK))
                                    .color(theme.success),
                            );
                        } else {
                            ui.label(typography::label("Not Installed").color(theme.text_disabled));
                        }
                    });
                });

                ui.add_space(spacing::SPACING_SM);

                let install_cmd = "curl -fsSL https://d2lang.com/install.sh | sh -s --";
                let uninstall_cmd =
                    "curl -fsSL https://d2lang.com/install.sh | sh -s -- --uninstall";

                if d2_installed {
                    ui.label(typography::body("D2 is ready to render diagrams."));
                    ui.add_space(spacing::SPACING_MD);

                    ui.collapsing(typography::label("Uninstall Options"), |ui| {
                        ui.add_space(spacing::SPACING_SM);
                        self.ui_copyable_command(ui, "Manual Uninstall", uninstall_cmd);

                        ui.add_space(spacing::SPACING_SM);
                        let btn = egui::Button::new(typography::label("Run Uninstall Script"))
                            .fill(theme.bg_card);
                        if ui.add_enabled(!self.state.ui.is_d2_installing, btn).clicked() {
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
                                ui.label(
                                    typography::body(icons::ICON_WARNING)
                                        .color(theme.warning)
                                        .size(16.0),
                                );
                                ui.vertical(|ui| {
                                    ui.label(typography::bold("Remote Script Warning"));
                                    ui.label(typography::body("Installation requires running a remote shell script. You can run it manually or allow LaReview to run it."));
                                });
                            });
                        });

                    ui.add_space(spacing::SPACING_MD);

                    let mut allow = self.state.ui.allow_d2_install;
                    if ui
                        .checkbox(
                            &mut allow,
                            typography::label("I understand and want to proceed"),
                        )
                        .changed()
                    {
                        self.dispatch(Action::Settings(SettingsAction::SetAllowD2Install(allow)));
                    }

                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        let can_install =
                            self.state.ui.allow_d2_install && !self.state.ui.is_d2_installing;
                        if ui
                            .add_enabled(
                                can_install,
                                egui::Button::new(typography::label("Install Automatically")),
                            )
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

                if self.state.ui.is_d2_installing {
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        crate::ui::animations::cyber::cyber_spinner(
                            ui,
                            theme.brand,
                            Some(crate::ui::animations::cyber::CyberSpinnerSize::Sm),
                        );
                        ui.label(typography::body("Processing..."));
                    });
                }

                if !self.state.ui.d2_install_output.is_empty() {
                    ui.add_space(12.0);
                    egui::CollapsingHeader::new(typography::bold("Script Output Log"))
                        .default_open(true)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .max_height(150.0)
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(
                                            &mut self.state.ui.d2_install_output.as_str(),
                                        )
                                        .font(typography::mono_font(13.0))
                                        .desired_width(f32::INFINITY)
                                        .lock_focus(true),
                                    );
                                });
                        });
                }
            });
    }
}
