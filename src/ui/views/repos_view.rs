use crate::ui::app::{Action, LaReviewApp, SettingsAction};
use crate::ui::spacing;
use crate::ui::theme;
use eframe::egui;

impl LaReviewApp {
    pub fn ui_repos(&mut self, ui: &mut egui::Ui) {
        ui.heading("Repositories");
        ui.add_space(spacing::SPACING_LG);

        let theme = theme::current_theme();

        // --- Linked Repositories Section ---
        egui::Frame::group(ui.style())
            .fill(theme.bg_secondary)
            .inner_margin(spacing::SPACING_LG)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong("Linked Repositories");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("➕ Link Repository").clicked() {
                            self.dispatch(Action::Settings(SettingsAction::LinkRepository));
                        }
                    });
                });

                ui.add_space(spacing::SPACING_MD);

                if self.state.linked_repos.is_empty() {
                    ui.weak("No repositories linked. Link a local Git repo to allow the agent to read file contents.");
                } else {
                    for repo in self.state.linked_repos.clone() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new(&repo.name).strong());
                                    ui.monospace(repo.path.to_string_lossy());
                                });

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("✖ Unlink").clicked() {
                                        self.dispatch(Action::Settings(SettingsAction::UnlinkRepository(repo.id.clone())));
                                    }
                                });
                            });

                            if !repo.remotes.is_empty() {
                                ui.add_space(spacing::SPACING_XS);
                                ui.horizontal(|ui| {
                                    ui.weak("Remotes: ");
                                    for remote in &repo.remotes {
                                        ui.label(egui::RichText::new(remote).small().color(theme.text_disabled));
                                    }
                                });
                            }
                        });
                        ui.add_space(spacing::SPACING_SM);
                    }
                }
            });
    }
}
