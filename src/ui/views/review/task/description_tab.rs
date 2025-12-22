use crate::ui::app::LaReviewApp;
use crate::ui::spacing;
use crate::ui::theme::current_theme;
use eframe::egui;

impl LaReviewApp {
    pub(crate) fn render_description_tab(
        &mut self,
        ui: &mut egui::Ui,
        task: &crate::domain::ReviewTask,
    ) {
        egui::Frame::NONE
            .inner_margin(spacing::SPACING_LG) // Reduced from XL
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 16.0;

                    let description = crate::infra::normalize_newlines(&task.description);

                    ui.scope(|ui| {
                        ui.spacing_mut().item_spacing.y = 16.0;
                        ui.spacing_mut().indent = 0.0; // Removed extra indent

                        // Removed inner frame with large right margin
                        crate::ui::components::render_markdown(ui, &description);

                        if let Some(insight) = &task.insight {
                            ui.add_space(spacing::SPACING_XL);

                            egui::Frame::NONE
                                .fill(current_theme().bg_tertiary)
                                .inner_margin(egui::Margin::symmetric(
                                    spacing::SPACING_LG as i8,
                                    spacing::SPACING_MD as i8,
                                ))
                                .stroke(egui::Stroke::new(
                                    1.0,
                                    current_theme().warning.gamma_multiply(0.3),
                                ))
                                .corner_radius(crate::ui::spacing::RADIUS_LG)
                                .show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = 12.0;

                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("Insight")
                                                    .strong()
                                                    .size(13.0)
                                                    .color(current_theme().warning),
                                            );
                                        });

                                        ui.add_space(spacing::SPACING_XS);

                                        let insight_text =
                                            crate::infra::normalize_newlines(insight);
                                        crate::ui::components::render_markdown(ui, &insight_text);
                                    });
                                });
                            ui.add_space(spacing::SPACING_XL);
                        }
                    });
                });
            });
    }
}
