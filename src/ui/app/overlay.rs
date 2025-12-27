use super::{Action, LaReviewApp, ReviewAction, SettingsAction};
use crate::ui::components::DiffAction;
use crate::ui::components::pills::pill_action_button;
use crate::ui::{icons, spacing, typography};
use eframe::egui;

impl LaReviewApp {
    pub(super) fn render_full_diff_overlay(&mut self, ctx: &egui::Context) {
        let Some(full) = self.state.ui.full_diff.clone() else {
            return;
        };

        let viewport_rect = ctx.input(|i| i.viewport().inner_rect).unwrap_or_else(|| {
            let rect = ctx.available_rect();
            egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), rect.size())
        });

        if viewport_rect.width() < 100.0 || viewport_rect.height() < 100.0 {
            return;
        }

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
                    if ui.button(format!("{} Close", icons::ACTION_BACK)).clicked() {
                        self.dispatch(Action::Review(ReviewAction::CloseFullDiff));
                    }
                });

                ui.separator();

                let action = crate::ui::components::diff::render_diff_editor_full_view(
                    ui, &full.text, "diff",
                );
                if let DiffAction::OpenInEditor {
                    file_path,
                    line_number,
                } = action
                {
                    self.dispatch(Action::Review(ReviewAction::OpenInEditor {
                        file_path,
                        line_number,
                    }));
                }
            });

        if !open {
            self.dispatch(Action::Review(ReviewAction::CloseFullDiff));
        }
    }

    pub(super) fn render_export_preview_overlay(&mut self, ctx: &egui::Context) {
        let Some(mut preview) = self.state.ui.export_preview.clone() else {
            return;
        };

        let mut open = true;
        let viewport_rect = ctx.input(|i| i.viewport().inner_rect).unwrap_or_else(|| {
            let rect = ctx.available_rect();
            egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), rect.size())
        });

        if viewport_rect.width() < 100.0 || viewport_rect.height() < 100.0 {
            return;
        }

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
                    ui.label(typography::h1("Export Review Preview"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(spacing::SPACING_MD);
                        if ui
                            .add(egui::Button::new(icons::ACTION_CLOSE).frame(false))
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
                                ui.label(typography::bold("Edit Markdown:"));
                                ui.add(
                                    egui::TextEdit::multiline(&mut preview)
                                        .font(typography::mono_font(13.0))
                                        .desired_width(f32::INFINITY)
                                        .frame(false),
                                );
                            });

                            cols[1].vertical(|ui| {
                                ui.label(typography::bold("Preview:"));
                                // Register all generated assets so the preview can find them
                                for (uri, bytes) in &self.state.ui.export_assets {
                                    ui.ctx().include_bytes(uri.clone(), bytes.clone());
                                }

                                crate::ui::components::render_markdown(ui, &preview);
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
                        icons::ACTION_REFRESH,
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
                                    typography::bold("Save Review...")
                                        .color(crate::ui::theme::current_theme().bg_primary),
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
        // If we dispatched CloseExportPreview above, self.state.ui.export_preview might have been set to None.
        // We must not overwrite it back to Some(preview) here.
        if let Some(current_state_preview) = &self.state.ui.export_preview
            && current_state_preview != &preview
        {
            self.state.ui.export_preview = Some(preview);
        }

        if !open {
            self.dispatch(Action::Review(ReviewAction::CloseExportPreview));
        }
    }

    pub(super) fn render_requirements_overlay(&mut self, ctx: &egui::Context) {
        if !self.state.ui.show_requirements_modal {
            return;
        }

        let theme = crate::ui::theme::current_theme();
        let mut open = true;

        let gh_path = crate::infra::shell::find_bin("gh");
        let d2_path = crate::infra::shell::find_bin("d2");
        let agents = crate::infra::acp::list_agent_candidates();

        egui::Window::new("Setup Checklist")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .frame(
                egui::Frame::window(&ctx.style())
                    .inner_margin(egui::Margin::same(spacing::SPACING_MD as i8)),
            )
            .show(ctx, |ui| {
                ui.label("Ensure these tools are installed and discoverable:");
                ui.add_space(spacing::SPACING_SM);

                egui::Grid::new("requirements_grid")
                    .num_columns(3)
                    .spacing([spacing::SPACING_LG, spacing::SPACING_SM])
                    .show(ui, |ui| {
                        ui.label(typography::bold("Tool"));
                        ui.label(typography::bold("Status"));
                        ui.label(typography::bold("Path"));
                        ui.end_row();

                        render_requirement_row(ui, "GitHub CLI (gh)", &gh_path, theme);
                        render_requirement_row(ui, "D2", &d2_path, theme);

                        for agent in &agents {
                            let label = format!("Agent: {}", agent.label);
                            let path = agent.command.as_ref().map(std::path::PathBuf::from);
                            render_requirement_row(ui, &label, &path, theme);
                        }
                    });

                ui.add_space(spacing::SPACING_MD);
                ui.horizontal(|ui| {
                    if ui.button("Open Settings").clicked() {
                        self.switch_to_settings();
                        self.dispatch(Action::Settings(SettingsAction::DismissRequirements));
                    }
                    if ui.button("Dismiss").clicked() {
                        self.dispatch(Action::Settings(SettingsAction::DismissRequirements));
                    }
                });
            });

        if !open {
            self.dispatch(Action::Settings(SettingsAction::DismissRequirements));
        }
    }
}

impl LaReviewApp {
    pub(super) fn render_editor_picker_overlay(&mut self, ctx: &egui::Context) {
        if !self.state.ui.show_editor_picker {
            return;
        }

        let theme = crate::ui::theme::current_theme();
        let mut open = true;
        let editors = crate::infra::editor::list_available_editors();

        egui::Window::new("Choose Editor")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .frame(
                egui::Frame::window(&ctx.style())
                    .inner_margin(egui::Margin::same(spacing::SPACING_MD as i8)),
            )
            .show(ctx, |ui| {
                ui.label("Select a text editor to open files:");
                ui.add_space(spacing::SPACING_SM);

                if let Some(err) = &self.state.ui.editor_picker_error {
                    ui.label(typography::body(err).color(theme.destructive));
                    ui.add_space(spacing::SPACING_SM);
                }

                if editors.is_empty() {
                    ui.label(typography::body("No supported editors found on PATH."));
                    ui.label(typography::weak(
                        "Install one (VS Code, Cursor, Sublime, JetBrains) or add it to PATH.",
                    ));
                } else {
                    for editor in editors {
                        let label = format!("{} ({})", editor.label, editor.path.display());
                        if ui
                            .add_sized(
                                [ui.available_width(), 28.0],
                                egui::Button::new(typography::label(label)),
                            )
                            .clicked()
                        {
                            self.dispatch(Action::Settings(SettingsAction::SetPreferredEditor(
                                editor.id.to_string(),
                            )));
                        }
                    }
                }

                ui.add_space(spacing::SPACING_MD);
                if ui.button("Cancel").clicked() {
                    self.dispatch(Action::Settings(SettingsAction::CloseEditorPicker));
                }
            });

        if !open {
            self.dispatch(Action::Settings(SettingsAction::CloseEditorPicker));
        }
    }
}

fn render_requirement_row(
    ui: &mut egui::Ui,
    label: &str,
    path: &Option<std::path::PathBuf>,
    theme: crate::ui::theme::Theme,
) {
    let is_ready = path.is_some();
    ui.label(label);
    if is_ready {
        ui.colored_label(theme.success, "✔ Ready");
    } else {
        ui.colored_label(theme.destructive, "✖ Missing");
    }
    ui.monospace(
        path.as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "Not found".to_string()),
    );
    ui.end_row();
}
