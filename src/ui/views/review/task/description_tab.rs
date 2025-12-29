use crate::ui::app::LaReviewApp;
use crate::ui::theme::current_theme;
use crate::ui::{spacing, typography};
use eframe::egui;
use std::sync::Arc;

fn get_or_normalize_text(
    ui: &egui::Ui,
    task_id: &str,
    field_name: &str,
    original: &str,
) -> Arc<str> {
    // Cache key includes content hash to invalidate when content changes
    let content_hash = egui::util::hash(original);
    let cache_key = egui::Id::new(("norm_text", task_id, field_name, content_hash));

    ui.ctx().memory_mut(|mem| {
        let entry = mem
            .data
            .get_temp_mut_or_default::<(bool, Arc<str>)>(cache_key);
        if !entry.0 {
            entry.0 = true;
            entry.1 = Arc::from(crate::infra::normalize_newlines(original).as_str());
        }
        entry.1.clone()
    })
}

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

                    let description =
                        get_or_normalize_text(ui, &task.id, "description", &task.description);

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
                                                typography::bold("Insight")
                                                    .size(13.0)
                                                    .color(current_theme().warning),
                                            );
                                        });

                                        ui.add_space(spacing::SPACING_XS);

                                        let insight_text =
                                            get_or_normalize_text(ui, &task.id, "insight", insight);
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
