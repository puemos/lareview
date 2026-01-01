use crate::ui::app::{Action, LaReviewApp, SettingsAction};
use crate::ui::components::pills::pill_action_button;
use crate::ui::icons;
use crate::ui::spacing::{self, TOP_HEADER_HEIGHT};
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
            .inner_margin(egui::Margin::symmetric(spacing::SPACING_XL as i8, 0))
            .show(ui, |ui| {
                ui.set_min_height(TOP_HEADER_HEIGHT);
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), TOP_HEADER_HEIGHT),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.horizontal(|ui| ui.label(typography::h2("Linked Repositories")));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(spacing::SPACING_XS);

                            if pill_action_button(
                                ui,
                                icons::ICON_PLUS,
                                "Add Repository",
                                true,
                                theme.border,
                            )
                            .clicked()
                            {
                                self.dispatch(Action::Settings(SettingsAction::LinkRepository));
                            }
                        });
                    },
                );
            });

        ui.separator();

        // --- Content Section ---
        if self.state.domain.linked_repos.is_empty() {
            egui::Frame::NONE
                .inner_margin(egui::Margin::symmetric(
                    spacing::SPACING_XL as i8,
                    spacing::SPACING_LG as i8,
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
                        spacing::SPACING_XL as i8,
                        spacing::SPACING_LG as i8,
                    ))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                typography::body(icons::VIEW_REPOS)
                                    .size(16.0)
                                    .color(theme.text_primary),
                            );
                            ui.add_space(8.0);

                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 4.0;

                                ui.horizontal_centered(|ui| {
                                    ui.label(
                                        typography::body(&repo.name).color(theme.text_primary),
                                    );
                                    ui.label(typography::tiny("•"));

                                    ui.label(typography::tiny(repo.path.to_string_lossy()));
                                });
                                ui.horizontal(|ui| {
                                    if !repo.remotes.is_empty() {
                                        let remotes_str = repo.remotes.join(", ");
                                        ui.label(typography::weak(remotes_str));
                                    }
                                });
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let unlink_label = typography::body(format!(
                                        "{} Unlink Repository",
                                        icons::ACTION_TRASH
                                    ))
                                    .color(theme.destructive);
                                    if ui.button(unlink_label).clicked() {
                                        self.dispatch(Action::Settings(
                                            SettingsAction::UnlinkRepository(repo.id.clone()),
                                        ));
                                    }
                                },
                            );
                        });
                    });

                if index + 1 < total_repos {
                    ui.separator();
                }
            }
        }
    }
}
