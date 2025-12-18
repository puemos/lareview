use eframe::egui;

use super::{Action, LaReviewApp, ReviewAction};
use crate::ui::components::pills::pill_action_button;
use crate::ui::spacing;

impl LaReviewApp {
    pub(super) fn render_full_diff_overlay(&mut self, ctx: &egui::Context) {
        let Some(full) = self.state.full_diff.clone() else {
            return;
        };

        let viewport_rect = ctx.input(|i| i.viewport().inner_rect).unwrap_or_else(|| {
            let rect = ctx.available_rect();
            egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), rect.size())
        });

        let mut open = true;
        let outer_padding = egui::vec2(spacing::SPACING_MD, spacing::SPACING_SM);

        egui::Window::new(full.title.clone())
            .open(&mut open)
            .fixed_rect(viewport_rect.shrink2(outer_padding))
            .frame(
                egui::Frame::window(&ctx.style()).inner_margin(egui::Margin::symmetric(
                    spacing::SPACING_MD as i8,
                    spacing::SPACING_SM as i8,
                )),
            )
            .collapsible(false)
            .resizable(false)
            .title_bar(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(format!("{} Close", egui_phosphor::regular::ARROW_SQUARE_IN))
                        .clicked()
                    {
                        self.dispatch(Action::Review(ReviewAction::CloseFullDiff));
                    }
                });

                ui.separator();

                crate::ui::components::diff::render_diff_editor_full_view(ui, &full.text, "diff");
            });

        if !open {
            self.dispatch(Action::Review(ReviewAction::CloseFullDiff));
        }
    }

    pub(super) fn render_export_preview_overlay(&mut self, ctx: &egui::Context) {
        let Some(mut preview) = self.state.export_preview.clone() else {
            return;
        };

        let mut open = true;
        let viewport_rect = ctx.input(|i| i.viewport().inner_rect).unwrap_or_else(|| {
            let rect = ctx.available_rect();
            egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), rect.size())
        });
        let outer_padding = egui::vec2(spacing::SPACING_SM, spacing::SPACING_SM);

        egui::Window::new("Export Review Preview")
            .open(&mut open)
            .fixed_rect(viewport_rect.shrink2(outer_padding))
            .collapsible(false)
            .resizable(false)
            .title_bar(false) // Custom title bar
            .show(ctx, |ui| {
                // Custom Title Bar
                ui.horizontal(|ui| {
                    ui.add_space(spacing::SPACING_MD);
                    ui.label(
                        egui::RichText::new("Export Review Preview")
                            .heading()
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(spacing::SPACING_MD);
                        if ui
                            .add(egui::Button::new(egui_phosphor::regular::X).frame(false))
                            .clicked()
                        {
                            self.dispatch(Action::Review(ReviewAction::CloseExportPreview));
                        }
                    });
                });
                ui.add_space(spacing::SPACING_SM);
                ui.separator();

                let footer_height = 50.0;
                let scroll_height = ui.available_height() - footer_height - spacing::SPACING_MD;

                egui::ScrollArea::vertical()
                    .id_salt("export_preview_scroll")
                    .max_height(scroll_height)
                    .show(ui, |ui| {
                        ui.columns(2, |cols| {
                            cols[0].vertical(|ui| {
                                ui.label(egui::RichText::new("Edit Markdown:").strong());
                                ui.add(
                                    egui::TextEdit::multiline(&mut preview)
                                        .font(egui::TextStyle::Monospace)
                                        .desired_width(f32::INFINITY)
                                        .frame(false),
                                );
                            });

                            cols[1].vertical(|ui| {
                                ui.label(egui::RichText::new("Preview:").strong());
                                // Register all generated assets so the preview can find them
                                for (uri, bytes) in &self.state.export_assets {
                                    ui.ctx().include_bytes(uri.clone(), bytes.clone());
                                }

                                egui_commonmark::CommonMarkViewer::new().show(
                                    ui,
                                    &mut self.state.markdown_cache,
                                    &preview,
                                );
                            });
                        });
                    });

                ui.separator();
                ui.add_space(spacing::SPACING_SM);

                // Footer
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add_space(spacing::SPACING_MD);
                    // Preview refresh button
                    let resp = pill_action_button(
                        ui,
                        egui_phosphor::regular::ARROW_CLOCKWISE,
                        "Regenerate Preview",
                        true,
                        crate::ui::theme::current_theme().border,
                    )
                    .on_hover_text("Regenerate the preview from current data.");
                    if resp.clicked() {
                        self.dispatch(Action::Review(ReviewAction::RequestExportPreview));
                    }

                    // Existing cancel button
                    if ui.button("Cancel").clicked() {
                        self.dispatch(Action::Review(ReviewAction::CloseExportPreview));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(spacing::SPACING_MD);
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Save Review...")
                                        .color(crate::ui::theme::current_theme().bg_primary)
                                        .strong(),
                                )
                                .fill(crate::ui::theme::current_theme().brand),
                            )
                            .clicked()
                            && let Some(path) = rfd::FileDialog::new()
                                .add_filter("Markdown", &["md"])
                                .set_file_name("review_export.md")
                                .save_file()
                        {
                            self.dispatch(Action::Review(ReviewAction::ExportReviewToFile {
                                path,
                            }));
                        }
                    });
                });
            });

        // CRITICAL: Only update state if it's still "open".
        // If we dispatched CloseExportPreview above, self.state.export_preview might have been set to None.
        // We must not overwrite it back to Some(preview) here.
        if let Some(current_state_preview) = &self.state.export_preview
            && current_state_preview != &preview
        {
            self.state.export_preview = Some(preview);
        }

        if !open {
            self.dispatch(Action::Review(ReviewAction::CloseExportPreview));
        }
    }
}
