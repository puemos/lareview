use super::{Action, LaReviewApp, ReviewAction, SettingsAction};
use crate::ui::components::DiffAction;
use crate::ui::components::pills::pill_action_button;
use crate::ui::theme::{Theme, current_theme};
use crate::ui::{icons, spacing, typography};
use eframe::egui;
use egui::Margin;

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
            .id(egui::Id::new("full_diff_overlay"))
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
        if !self.state.ui.is_exporting && self.state.ui.export_preview.is_none() {
            return;
        }

        let preview_state = self.state.ui.export_preview.clone();

        let mut open = true;
        let viewport_rect = ctx.input(|i| i.viewport().inner_rect).unwrap_or_else(|| {
            let rect = ctx.available_rect();
            egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), rect.size())
        });

        if viewport_rect.width() < 100.0 || viewport_rect.height() < 100.0 {
            return;
        }

        let theme = current_theme();
        let review_title = self
            .state
            .ui
            .selected_review_id
            .as_ref()
            .and_then(|id| self.state.domain.reviews.iter().find(|r| &r.id == id))
            .map(|r| r.title.clone())
            .unwrap_or_else(|| "Review".to_string());

        egui::Window::new("Export Review Window")
            .id(egui::Id::new("export_review_overlay"))
            .open(&mut open)
            .fixed_rect(viewport_rect) // 100% width and height
            .collapsible(false)
            .resizable(false)
            .title_bar(false) // Custom title bar
            .frame(egui::Frame::window(&ctx.style()).inner_margin(0.0)) // No margin for touching lines
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                    // --- 1. Header (Title Bar) ---
                    egui::Frame::NONE
                        .inner_margin(Margin {
                            left: spacing::SPACING_MD as i8,
                            right: spacing::SPACING_MD as i8,
                            top: spacing::SPACING_SM as i8,
                            bottom: spacing::SPACING_SM as i8,
                        })
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(typography::h1(format!("{} Export Review", icons::ACTION_EXPORT)));
                                ui.add_space(spacing::SPACING_SM);
                                ui.label(typography::body(&review_title).size(18.0).color(theme.text_muted));

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui
                                        .add(egui::Button::new(icons::ACTION_CLOSE).frame(false))
                                        .clicked()
                                    {
                                        self.dispatch(Action::Review(ReviewAction::CloseExportPreview));
                                    }
                                });
                            });
                        });

                    ui.separator(); // Full width separator

                    let footer_height = 60.0;
                    let available_height = ui.available_height() - footer_height - 1.0; // -1 for separator

                    // --- 2. Main Body (Sidebar + Preview) ---
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), available_height),
                        egui::Layout::left_to_right(egui::Align::Min),
                        |ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                            let mut sidebar_width = if self.state.ui.export_sidebar_width > 50.0 {
                                self.state.ui.export_sidebar_width
                            } else {
                                300.0
                            };

                            // A. Sidebar
                            ui.push_id("export_sidebar", |ui| {
                                ui.allocate_ui_with_layout(
                                    egui::vec2(sidebar_width, available_height),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        ui.set_max_width(sidebar_width);
                                        ui.set_max_height(available_height);

                                        egui::ScrollArea::vertical()
                                            .id_salt("export_sidebar_scroll")
                                            .show(ui, |ui| {
                                                // Options Section (Open by Default)
                                                ui.add_space(spacing::SPACING_SM);

                                                let id = ui.make_persistent_id("export_options_collapsed");
                                                egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true).show_header(ui, |ui| {
                                                    ui.label(typography::bold("Options"));
                                                }).body(|ui| {
                                                    let mut options = self.state.ui.export_options.clone();
                                                    let mut changed = false;

                                                    fn icon_checkbox(ui: &mut egui::Ui, theme: &Theme, value: &mut bool, label: &str) -> egui::Response {
                                                        ui.horizontal(|ui| {
                                                            let icon = if *value { icons::ICON_CHECK_SQUARE } else { icons::ICON_SQUARE };
                                                            let resp = ui.label(typography::body(format!("{} {}", icon, label)).color(theme.text_primary))
                                                                .interact(egui::Sense::click());
                                                            if resp.clicked() {
                                                                *value = !*value;
                                                            }
                                                            resp
                                                        }).inner
                                                    }

                                                    ui.add_space(spacing::SPACING_XS);
                                                    if icon_checkbox(ui, &theme, &mut options.include_summary, "Include Summary").clicked() { changed = true; }
                                                    if icon_checkbox(ui, &theme, &mut options.include_stats, "Include Stats").clicked() { changed = true; }
                                                    if icon_checkbox(ui, &theme, &mut options.include_metadata, "Include Metadata").clicked() { changed = true; }
                                                    if icon_checkbox(ui, &theme, &mut options.include_tasks, "Include Tasks").clicked() { changed = true; }
                                                    if icon_checkbox(ui, &theme, &mut options.include_threads, "Include Threads").clicked() { changed = true; }

                                                    if changed {
                                                        self.dispatch(Action::Review(ReviewAction::UpdateExportOptions(options)));
                                                    }
                                                    ui.add_space(spacing::SPACING_SM);
                                                });

                                                ui.separator();

                                                // Threads Section
                                                ui.add_space(spacing::SPACING_SM);
                                                ui.horizontal(|ui| {
                                                    ui.add_space(spacing::SPACING_MD);
                                                    ui.label(typography::bold("Feedback Threads"));
                                                });
                                                ui.add_space(spacing::SPACING_SM);

                                                // Select All / Clear All
                                                ui.horizontal(|ui| {
                                                    ui.add_space(spacing::SPACING_MD);
                                                    if pill_action_button(ui, icons::ICON_CHECK, "Select All", true, theme.border).clicked() {
                                                        self.dispatch(Action::Review(ReviewAction::SelectAllExportThreads));
                                                    }
                                                    ui.add_space(spacing::SPACING_XS);
                                                    if pill_action_button(ui, icons::ACTION_CLOSE, "Clear", true, theme.border).clicked() {
                                                        self.dispatch(Action::Review(ReviewAction::ClearExportThreads));
                                                    }
                                                });
                                                ui.add_space(spacing::SPACING_SM);

                                                let review_threads = self.state.domain.threads.clone();

                                                ui.scope(|ui| {
                                                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                                                    if let Some(action) = crate::ui::views::review::thread_list::render_thread_list(
                                                        ui,
                                                        &review_threads,
                                                        None,
                                                        true,
                                                        &self.state.ui.export_options.selected_thread_ids,
                                                        false,
                                                        &theme,
                                                    ) {
                                                        self.dispatch(Action::Review(action));
                                                    }
                                                });
                                            });
                                    }
                                );
                            });

                            // B. Vertical Resize Handle
                            let handle_rect = egui::Rect::from_min_max(
                                egui::pos2(ui.min_rect().min.x + sidebar_width - 2.0, ui.min_rect().min.y),
                                egui::pos2(ui.min_rect().min.x + sidebar_width + 2.0, ui.min_rect().min.y + available_height)
                            );
                            let resp = ui.allocate_rect(handle_rect, egui::Sense::drag());

                            let painter = ui.painter();
                            let stroke_color = if resp.hovered() || resp.dragged() {
                                theme.brand
                            } else {
                                theme.border
                            };
                            painter.vline(handle_rect.center().x, handle_rect.y_range(), egui::Stroke::new(1.0, stroke_color));

                            if resp.dragged() {
                                sidebar_width += resp.drag_delta().x;
                                sidebar_width = sidebar_width.clamp(200.0, 600.0);
                                self.state.ui.export_sidebar_width = sidebar_width;
                            }

                            if resp.hovered() || resp.dragged() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                            }

                            // C. Main Preview Area
                            ui.push_id("export_preview", |ui| {
                                let preview_rect = ui.available_rect_before_wrap();
                                ui.scope_builder(egui::UiBuilder::new().max_rect(preview_rect), |ui| {
                                    egui::Frame::NONE
                                        .inner_margin(Margin::same(12))
                                        .show(ui, |ui| {
                                            ui.vertical(|ui| {
                                                ui.label(typography::bold("Preview"));
                                                ui.add_space(spacing::SPACING_SM);

                                                egui::ScrollArea::vertical()
                                                    .id_salt("export_preview_scroll")
                                                    .show(ui, |ui| {
                                                        if self.state.ui.is_exporting {
                                                            ui.vertical_centered(|ui| {
                                                                ui.add_space(spacing::SPACING_XL);
                                                                crate::ui::animations::cyber::cyber_spinner(
                                                                    ui,
                                                                    theme.brand,
                                                                    Some(crate::ui::animations::cyber::CyberSpinnerSize::Md)
                                                                );
                                                                ui.add_space(spacing::SPACING_MD);
                                                                ui.label(typography::body("Generating preview...").color(theme.text_muted));
                                                            });
                                                        } else if let Some(p) = self.state.ui.export_preview.as_ref() {
                                                            crate::ui::components::render_markdown(ui, p);
                                                        }
                                                    });
                                            });
                                        });
                                });
                            });
                        },
                    );

                    ui.separator(); // Footer separator

                    // --- 3. Footer ---
                    egui::Frame::NONE
                        .inner_margin(Margin {
                            left: spacing::SPACING_MD as i8,
                            right: spacing::SPACING_MD as i8,
                            top: spacing::SPACING_SM as i8,
                            bottom: spacing::SPACING_SM as i8,
                        })
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // Regenerate
                                if pill_action_button(ui, icons::ACTION_REFRESH, "Regenerate", true, theme.border).clicked() {
                                    self.dispatch(Action::Review(ReviewAction::RequestExportPreview));
                                }

                                ui.add_space(spacing::SPACING_XS);

                                // Cancel
                                if pill_action_button(ui, icons::ACTION_CLOSE, "Cancel", true, theme.border).clicked() {
                                    self.dispatch(Action::Review(ReviewAction::CloseExportPreview));
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    // Save
                                    if pill_action_button(ui, icons::ACTION_SAVE, "Save Review...", true, theme.brand).clicked() && let Some(path) = rfd::FileDialog::new()
                                        .add_filter("Markdown", &["md"])
                                        .set_file_name("review_export.md")
                                        .save_file()
                                        {
                                            self.dispatch(Action::Review(ReviewAction::ExportReviewToFile { path }));
                                        }
                                });
                            });
                        });
                });
            });

        // Sync preview state
        if let Some(current_state_preview) = &self.state.ui.export_preview
            && let Some(new_preview) = &preview_state
            && current_state_preview != new_preview
        {
            self.state.ui.export_preview = Some(new_preview.clone());
        }

        if !open {
            self.dispatch(Action::Review(ReviewAction::CloseExportPreview));
        }
    }

    pub(super) fn render_requirements_overlay(&mut self, ctx: &egui::Context) {
        if !self.state.ui.show_requirements_modal {
            return;
        }

        let theme = current_theme();
        let mut open = true;

        let gh_path = crate::infra::shell::find_bin("gh");
        let d2_path = crate::infra::shell::find_bin("d2");
        let agents = crate::infra::acp::list_agent_candidates();

        egui::Window::new("Setup Checklist")
            .id(egui::Id::new("setup_checklist_overlay"))
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

                egui::Grid::new(ui.make_persistent_id("requirements_grid"))
                    .num_columns(3)
                    .spacing([spacing::SPACING_LG, spacing::SPACING_SM])
                    .show(ui, |ui| {
                        ui.label(typography::bold("Tool"));
                        ui.label(typography::bold("Status"));
                        ui.label(typography::bold("Path"));
                        ui.end_row();

                        render_requirement_row(ui, "GitHub CLI (gh)", &gh_path, &theme);
                        render_requirement_row(ui, "D2", &d2_path, &theme);

                        for agent in &agents {
                            let label = format!("Agent: {}", agent.label);
                            let path = agent.command.as_ref().map(std::path::PathBuf::from);
                            render_requirement_row(ui, &label, &path, &theme);
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

    pub(super) fn render_editor_picker_overlay(&mut self, ctx: &egui::Context) {
        if !self.state.ui.show_editor_picker {
            return;
        }

        let theme = current_theme();
        let mut open = true;
        let editors = crate::infra::editor::list_available_editors();

        egui::Window::new("Choose Editor")
            .id(egui::Id::new("choose_editor_overlay"))
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
    theme: &Theme,
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
