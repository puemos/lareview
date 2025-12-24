use crate::domain::{Review, ReviewSource};
use crate::infra::acp::{AgentCandidate, list_agent_candidates};
use crate::ui::app::{Action, AppView, LaReviewApp, NavigationAction, ReviewAction};
use crate::ui::components::pills::pill_action_button;
use crate::ui::icons;
use crate::ui::spacing::{self, TOP_HEADER_HEIGHT};
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
                        egui::Frame::NONE
                            .inner_margin(egui::Margin::symmetric(spacing::SPACING_XL as i8, 0))
                            .show(ui, |ui| {
                                ui.set_min_height(TOP_HEADER_HEIGHT);
                                ui.allocate_ui_with_layout(
                                    egui::vec2(ui.available_width(), TOP_HEADER_HEIGHT),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        // A. Left Side: Context Selectors
                                        ui.horizontal(|ui| {
                                            ui.label(typography::h2("Recent Reviews"))
                                        });

                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                // C. Right Side: Actions
                                                ui.add_space(spacing::SPACING_XS);

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
                                                        NavigationAction::SwitchTo(
                                                            AppView::Generate,
                                                        ),
                                                    ));
                                                }
                                            },
                                        );
                                    },
                                );
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
                                    ui.label(typography::weak(
                                        "No reviews yet. Start one to see it here.",
                                    ));
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

                        ui.separator();

                        ui.add_space(spacing::SPACING_XL);

                        // --- 3. Agents Section ---
                        egui::Frame::NONE
                            .inner_margin(egui::Margin::symmetric(
                                spacing::SPACING_XL as i8,
                                spacing::SPACING_SM as i8,
                            ))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        typography::bold("Available Agents")
                                            .color(theme.text_primary),
                                    );
                                });
                            });

                        // --- 4. Agents List ---
                        let mut agents = list_agent_candidates();
                        agents.sort_by(|a, b| b.available.cmp(&a.available));

                        if agents.is_empty() {
                            egui::Frame::NONE
                                .inner_margin(egui::Margin::symmetric(
                                    spacing::SPACING_XL as i8,
                                    spacing::SPACING_LG as i8,
                                ))
                                .show(ui, |ui| {
                                    ui.label(typography::weak("No agents discovered."));
                                });
                        } else {
                            egui::Frame::NONE
                                .inner_margin(egui::Margin::symmetric(
                                    spacing::SPACING_XL as i8,
                                    spacing::SPACING_SM as i8,
                                ))
                                .show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        let total = agents.len();
                                        for (i, agent) in agents.iter().enumerate() {
                                            self.render_simple_agent_row(ui, agent);
                                            if i + 1 < total {
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

        // Calculate stats
        let active_run_id = review.active_run_id.as_ref();
        let tasks: Vec<_> = self
            .state
            .domain
            .all_tasks
            .iter()
            .filter(|t| Some(&t.run_id) == active_run_id)
            .collect();

        let total_tasks = tasks.len();
        let done_tasks = tasks.iter().filter(|t| t.status.is_closed()).count();

        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(
                spacing::SPACING_XL as i8,
                spacing::SPACING_LG as i8,
            ))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // 1. Icon (Far Left)
                    let icon = match review.source {
                        ReviewSource::GitHubPr { .. } => icons::ICON_GITHUB,
                        ReviewSource::DiffPaste { .. } => icons::ICON_FILES,
                    };
                    ui.label(typography::body(icon).size(16.0).color(theme.text_primary));
                    ui.add_space(8.0);

                    // 2. Content Column (Title, Subtitle, Tasks)
                    ui.vertical(|ui| {
                        let time_str = if let Ok(dt) =
                            chrono::DateTime::parse_from_rfc3339(&review.updated_at)
                        {
                            dt.format("%Y-%m-%d %H:%M").to_string()
                        } else {
                            review.updated_at.clone()
                        };

                        // Row 1: Title
                        ui.horizontal_centered(|ui| {
                            ui.label(typography::label(&review.title).color(theme.text_primary));
                            ui.label(typography::tiny("•"));
                            ui.label(typography::tiny(time_str));
                            let source_meta = match &review.source {
                                ReviewSource::GitHubPr {
                                    owner,
                                    repo,
                                    number,
                                    ..
                                } => Some(format!("{owner}/{repo}#{number}")),
                                ReviewSource::DiffPaste { .. } => None,
                            };
                            if let Some(source_meta) = source_meta {
                                ui.label(typography::weak("•"));
                                ui.label(typography::weak(source_meta));
                            }
                        });
                    });

                    // 3. Actions Column (Right Aligned)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Open Button
                        if ui.button("Open").clicked() {
                            self.dispatch(Action::Review(ReviewAction::SelectReview {
                                review_id: review.id.clone(),
                            }));
                            self.dispatch(Action::Navigation(NavigationAction::SwitchTo(
                                AppView::Review,
                            )));
                        }

                        // Delete Button
                        let delete_label =
                            typography::body(format!("{} Delete", icons::ACTION_TRASH))
                                .color(theme.destructive);
                        if ui.button(delete_label).clicked() {
                            self.dispatch(Action::Review(ReviewAction::DeleteReview(
                                review.id.clone(),
                            )));
                        }

                        // Progress Stats
                        if total_tasks > 0 {
                            ui.add_space(spacing::SPACING_MD);
                            ui.label(
                                typography::small(format!("{done_tasks}/{total_tasks} Tasks"))
                                    .color(if done_tasks == total_tasks {
                                        theme.success
                                    } else {
                                        theme.text_muted
                                    }),
                            );
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
                    typography::body(icons::VIEW_GENERATE)
                        .size(16.0)
                        .color(theme.text_disabled),
                );
            }

            ui.add_space(6.0);

            // 2. Name
            ui.label(typography::bold(&agent.label).color(theme.text_primary));

            // 3. Status
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if agent.available {
                    ui.label(typography::small("Ready").color(theme.success).strong());
                } else {
                    ui.label(typography::small("Unavailable").color(theme.text_disabled));
                }
            });
        });
    }
}
