use crate::domain::{Review, ReviewSource};
use crate::infra::acp::{list_agent_candidates, AgentCandidate};
use crate::ui::app::{Action, AppView, LaReviewApp, NavigationAction, ReviewAction};
use crate::ui::icons;
use crate::ui::spacing;
use crate::ui::theme::current_theme;
use eframe::egui;

impl LaReviewApp {
    pub fn ui_home(&mut self, ui: &mut egui::Ui) {
        if ui.available_width() < 100.0 {
            return;
        }

        let theme = current_theme();

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme.bg_primary))
            .show(ui.ctx(), |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        // --- 1. Header Section ---
                        egui::Frame::NONE
                            .inner_margin(egui::Margin::symmetric(
                                spacing::SPACING_LG as i8,
                                spacing::SPACING_MD as i8,
                            ))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.strong("Recent Reviews");
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let btn = egui::Button::new(
                                                egui::RichText::new(format!(
                                                    "{} New Review",
                                                    icons::ICON_PLUS
                                                ))
                                                .strong()
                                                .color(theme.bg_primary),
                                            )
                                            .fill(theme.brand)
                                            .corner_radius(6.0)
                                            .min_size(egui::vec2(100.0, 24.0));

                                            if ui.add(btn).clicked() {
                                                self.dispatch(Action::Navigation(
                                                    NavigationAction::SwitchTo(AppView::Generate),
                                                ));
                                            }
                                        },
                                    );
                                });
                            });

                        ui.separator();

                        // --- 2. Reviews List ---
                        let reviews = self.state.domain.reviews.clone();
                        if reviews.is_empty() {
                            egui::Frame::NONE
                                .inner_margin(egui::Margin::symmetric(
                                    spacing::SPACING_LG as i8,
                                    spacing::SPACING_MD as i8,
                                ))
                                .show(ui, |ui| {
                                    ui.weak("No reviews yet. Start one to see it here.");
                                });
                        } else {
                            let total_reviews = reviews.len();
                            for (index, review) in reviews.iter().enumerate() {
                                self.render_review_row(ui, review);

                                if index + 1 < total_reviews {
                                    ui.separator();
                                }
                            }
                        }

                        ui.add_space(spacing::SPACING_XL);

                        // --- 3. Agents Section ---
                        egui::Frame::NONE
                            .inner_margin(egui::Margin::symmetric(
                                spacing::SPACING_LG as i8,
                                spacing::SPACING_MD as i8,
                            ))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.strong("Available Agents");
                                });
                            });

                        ui.separator();

                        // --- 4. Agents List ---
                        let agents = list_agent_candidates();
                        if agents.is_empty() {
                            egui::Frame::NONE
                                .inner_margin(egui::Margin::symmetric(
                                    spacing::SPACING_LG as i8,
                                    spacing::SPACING_MD as i8,
                                ))
                                .show(ui, |ui| {
                                    ui.weak("No agents discovered.");
                                });
                        } else {
                            let total_agents = agents.len();
                            for (index, agent) in agents.iter().enumerate() {
                                self.render_agent_row(ui, agent);

                                if index + 1 < total_agents {
                                    ui.separator();
                                }
                            }
                        }
                    });
            });
    }

    fn render_review_row(&mut self, ui: &mut egui::Ui, review: &Review) {
        let theme = current_theme();

        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(
                spacing::SPACING_LG as i8,
                spacing::SPACING_MD as i8,
            ))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Icon
                    let icon = match review.source {
                        ReviewSource::GitHubPr { .. } => icons::ICON_GITHUB,
                        ReviewSource::DiffPaste { .. } => icons::ICON_FILES,
                    };
                    ui.label(
                        egui::RichText::new(icon)
                            .size(16.0)
                            .color(theme.text_secondary),
                    );
                    ui.add_space(8.0);

                    // Content
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(&review.title).strong());
                        
                        let time_str = if let Ok(dt) =
                            chrono::DateTime::parse_from_rfc3339(&review.updated_at)
                        {
                            dt.format("%Y-%m-%d %H:%M").to_string()
                        } else {
                            review.updated_at.clone()
                        };
                        
                        ui.horizontal(|ui| {
                             ui.label(
                                egui::RichText::new(&review.id)
                                    .size(10.0)
                                    .monospace()
                                    .color(theme.text_disabled),
                            );
                            ui.label(
                                egui::RichText::new(format!("• Updated {}", time_str))
                                    .size(11.0)
                                    .color(theme.text_muted),
                            );
                        });
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Delete Button
                         if ui.button("✖ Delete").clicked() {
                             self.dispatch(Action::Review(ReviewAction::DeleteReview(
                                review.id.clone(),
                            )));
                        }

                        // Open Button (Primary Action)
                        if ui.button("Open").clicked() {
                            self.dispatch(Action::Review(ReviewAction::SelectReview {
                                review_id: review.id.clone(),
                            }));
                            self.dispatch(Action::Navigation(NavigationAction::SwitchTo(
                                AppView::Review,
                            )));
                        }
                    });
                });
            });
    }

    fn render_agent_row(&self, ui: &mut egui::Ui, agent: &AgentCandidate) {
        let theme = current_theme();

        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(
                spacing::SPACING_LG as i8,
                spacing::SPACING_MD as i8,
            ))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Icon/Logo Placeholder
                    // Ideally we'd load the logo if available, but for now use generic icon
                    ui.label(
                        egui::RichText::new(icons::VIEW_GENERATE) // Using generic "AI/Diff" icon
                            .size(16.0)
                            .color(if agent.available { theme.brand } else { theme.text_disabled }),
                    );
                     ui.add_space(8.0);

                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(&agent.label).strong());
                         ui.label(
                            egui::RichText::new(&agent.id)
                                .size(11.0)
                                .monospace()
                                .color(theme.text_muted),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if agent.available {
                             ui.colored_label(theme.success, "✔ Available");
                        } else {
                             ui.colored_label(theme.text_disabled, "Not Found");
                        }
                        
                        if let Some(cmd) = &agent.command {
                             if ui.button("📋").on_hover_text("Copy Command").clicked() {
                                ui.ctx().copy_text(cmd.to_string());
                            }
                        }
                    });
                });
            });
    }
}