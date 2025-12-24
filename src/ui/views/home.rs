use crate::domain::{Review, ReviewSource};
use crate::infra::acp::{AgentCandidate, list_agent_candidates};
use crate::ui::app::{Action, AppView, LaReviewApp, NavigationAction, ReviewAction};
use crate::ui::components::pills::pill_action_button;
use crate::ui::icons;
use crate::ui::spacing;
use crate::ui::theme::current_theme;
use crate::ui::typography;
use eframe::egui;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

static LOGO_BYTES_CACHE: Lazy<Mutex<HashMap<String, Arc<[u8]>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn load_logo_bytes(path: &str) -> Option<Arc<[u8]>> {
    if let Ok(mut cache) = LOGO_BYTES_CACHE.lock() {
        if let Some(bytes) = cache.get(path) {
            return Some(bytes.clone());
        }

        let bytes: Arc<[u8]> = crate::assets::get_content(path)?.into();
        cache.insert(path.to_owned(), bytes.clone());
        Some(bytes)
    } else {
        crate::assets::get_content(path).map(Into::into)
    }
}

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
                        ui.add_space(spacing::SPACING_LG);

                        // --- 1. Header Section ---
                        egui::Frame::NONE
                            .inner_margin(egui::Margin::symmetric(
                                spacing::SPACING_XL as i8,
                                spacing::SPACING_LG as i8,
                            ))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        typography::bold("Recent Reviews")
                                            .color(theme.text_primary),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if pill_action_button(
                                                ui,
                                                icons::ICON_PLUS,
                                                "New Review",
                                                true,
                                                theme.border,
                                            )
                                            .clicked()
                                            {
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
                                    spacing::SPACING_XL as i8,
                                    spacing::SPACING_LG as i8,
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
                                spacing::SPACING_XL as i8,
                                spacing::SPACING_LG as i8,
                            ))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        typography::bold("Available Agents")
                                            .color(theme.text_primary),
                                    );
                                });
                            });

                        ui.separator();

                        // --- 4. Agents List (Simple) ---
                        let mut agents = list_agent_candidates();
                        // Sort by availability (Available first)
                        agents.sort_by(|a, b| b.available.cmp(&a.available));

                        if agents.is_empty() {
                            egui::Frame::NONE
                                .inner_margin(egui::Margin::symmetric(
                                    spacing::SPACING_XL as i8,
                                    spacing::SPACING_LG as i8,
                                ))
                                .show(ui, |ui| {
                                    ui.weak("No agents discovered.");
                                });
                        } else {
                            // Container Frame to match width/padding of other sections
                            egui::Frame::NONE
                                .inner_margin(egui::Margin::symmetric(
                                    spacing::SPACING_XL as i8,
                                    spacing::SPACING_LG as i8,
                                ))
                                .show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        let total_agents = agents.len();
                                        for (index, agent) in agents.iter().enumerate() {
                                            self.render_simple_agent_row(ui, agent);

                                            if index + 1 < total_agents {
                                                ui.add_space(spacing::SPACING_MD);
                                                ui.separator();
                                                ui.add_space(spacing::SPACING_MD);
                                            }
                                        }
                                    });
                                });
                        }

                        ui.add_space(spacing::SPACING_XL);
                    });
            });
    }

    fn render_review_row(&mut self, ui: &mut egui::Ui, review: &Review) {
        let theme = current_theme();

        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(
                spacing::SPACING_XL as i8,
                spacing::SPACING_LG as i8,
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
                        ui.label(typography::bold(&review.title));

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

    fn render_simple_agent_row(&self, ui: &mut egui::Ui, agent: &AgentCandidate) {
        let theme = current_theme();

        ui.horizontal(|ui| {
            // 1. Logo
            if let Some(logo_path) = &agent.logo
                && let Some(bytes) = load_logo_bytes(logo_path)
            {
                let uri = format!("bytes://{}", logo_path);
                let image = egui::Image::from_bytes(uri, bytes)
                    .fit_to_exact_size(egui::vec2(20.0, 20.0))
                    .corner_radius(4.0);

                if agent.available {
                    ui.add(image);
                } else {
                    ui.add(image.tint(egui::Color32::from_white_alpha(100)));
                }
            } else {
                ui.label(
                    egui::RichText::new(icons::VIEW_GENERATE)
                        .size(16.0)
                        .color(theme.text_disabled),
                );
            }

            ui.add_space(12.0);

            // 2. Name
            ui.label(typography::bold(&agent.label).color(theme.text_primary));

            // 3. Status
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if agent.available {
                    ui.label(egui::RichText::new("Ready").color(theme.success).size(11.0));
                } else {
                    ui.label(
                        egui::RichText::new("Unavailable")
                            .color(theme.text_disabled)
                            .size(11.0),
                    );
                }
            });
        });
    }
}
