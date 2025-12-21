use crate::ui::app::LaReviewApp;
use crate::ui::spacing::{self, SPACING_XL};
use crate::ui::theme::current_theme;
use eframe::egui;

impl LaReviewApp {
    pub(crate) fn render_description_tab(
        &mut self,
        ui: &mut egui::Ui,
        task: &crate::domain::ReviewTask,
    ) {
        egui::Frame::NONE
            .inner_margin(spacing::SPACING_XL)
            .show(ui, |ui| {
                let max_width = 720.0;
                let diff_width = ui.available_width() - max_width;
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 28.0;

                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.set_max_width(max_width);
                            let description = crate::infra::normalize_newlines(&task.description);

                            ui.scope(|ui| {
                                ui.style_mut().override_text_style = Some(egui::TextStyle::Body);

                                let body_font_id = egui::FontId::proportional(13.0);
                                ui.style_mut()
                                    .text_styles
                                    .insert(egui::TextStyle::Body, body_font_id);

                                let mono_font_id = egui::FontId::monospace(13.0);
                                ui.style_mut()
                                    .text_styles
                                    .insert(egui::TextStyle::Monospace, mono_font_id);

                                ui.spacing_mut().item_spacing.y = 12.0;
                                ui.spacing_mut().indent = 16.0;

                                ui.visuals_mut().override_text_color =
                                    Some(current_theme().text_secondary);
                                ui.visuals_mut().widgets.noninteractive.fg_stroke.color =
                                    current_theme().text_secondary;
                                ui.visuals_mut().extreme_bg_color = current_theme().bg_tertiary;
                                ui.visuals_mut().widgets.noninteractive.bg_fill =
                                    current_theme().bg_tertiary;

                                egui::Frame::NONE
                                    .inner_margin(egui::Margin {
                                        right: (SPACING_XL * 2.0) as i8,
                                        bottom: 0,
                                        left: 0,
                                        top: 0,
                                    })
                                    .show(ui, |ui| {
                                        egui_commonmark::CommonMarkViewer::new()
                                            .max_image_width(Some(max_width as usize))
                                            .show(ui, &mut self.state.markdown_cache, &description);
                                    });

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
                                                ui.spacing_mut().item_spacing.y = 16.0;

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
                                                ui.scope(|ui| {
                                                    ui.visuals_mut().override_text_color =
                                                        Some(current_theme().text_primary);
                                                    ui.visuals_mut()
                                                        .widgets
                                                        .noninteractive
                                                        .fg_stroke
                                                        .color = current_theme().text_primary;
                                                    ui.visuals_mut().extreme_bg_color =
                                                        current_theme().bg_surface;

                                                    let body_font_id =
                                                        egui::FontId::proportional(13.0);
                                                    ui.style_mut().text_styles.insert(
                                                        egui::TextStyle::Body,
                                                        body_font_id,
                                                    );

                                                    let mono_font_id =
                                                        egui::FontId::monospace(13.0);
                                                    ui.style_mut().text_styles.insert(
                                                        egui::TextStyle::Monospace,
                                                        mono_font_id,
                                                    );

                                                    egui_commonmark::CommonMarkViewer::new().show(
                                                        ui,
                                                        &mut self.state.markdown_cache,
                                                        &insight_text,
                                                    );
                                                });
                                            });
                                        });
                                    ui.add_space(spacing::SPACING_XL);
                                }
                            });
                        });
                        ui.allocate_space(egui::vec2(diff_width, 0.0));
                    });
                });
            });
    }
}
