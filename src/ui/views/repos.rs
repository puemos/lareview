use crate::ui::app::{Action, LaReviewApp, SettingsAction};
use crate::ui::spacing;
use crate::ui::{theme, typography};
use eframe::egui;

impl LaReviewApp {
    pub fn ui_repos(&mut self, ui: &mut egui::Ui) {
        if ui.available_width() < 100.0 {
            return;
        }

        let theme = theme::current_theme();

        // --- Header Section ---
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(
                spacing::SPACING_LG as i8,
                spacing::SPACING_MD as i8,
            ))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(typography::bold("Linked Repositories"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("➕ Link Repository").clicked() {
                            self.dispatch(Action::Settings(SettingsAction::LinkRepository));
                        }
                    });
                });
            });

        ui.separator();

        // --- Content Section ---
        if self.state.domain.linked_repos.is_empty() {
            egui::Frame::NONE
                .inner_margin(egui::Margin::symmetric(
                    spacing::SPACING_LG as i8,
                    spacing::SPACING_MD as i8,
                ))
                .show(ui, |ui| {
                    ui.label(typography::weak("No repositories linked. Link a local Git repo to allow the agent to read file contents."));
                });
        } else {
            let total_repos = self.state.domain.linked_repos.len();
            for (index, repo) in self
                .state
                .domain
                .linked_repos
                .clone()
                .into_iter()
                .enumerate()
            {
                egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(
                        spacing::SPACING_LG as i8,
                        spacing::SPACING_MD as i8,
                    ))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(typography::bold(&repo.name));
                                ui.label(typography::mono(repo.path.to_string_lossy()));
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("✖ Unlink").clicked() {
                                        self.dispatch(Action::Settings(
                                            SettingsAction::UnlinkRepository(repo.id.clone()),
                                        ));
                                    }
                                },
                            );
                        });

                        if !repo.remotes.is_empty() {
                            ui.add_space(spacing::SPACING_XS);
                            ui.horizontal(|ui| {
                                ui.label(typography::weak("Remotes: "));
                                for remote in &repo.remotes {
                                    ui.label(
                                        typography::small(remote)
                                            .color(theme.text_disabled),
                                    );
                                }
                            });
                        }
                    });

                if index + 1 < total_repos {
                    ui.separator();
                }
            }
        }
    }
}
