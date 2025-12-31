use crate::ui::app::ui_memory::with_ui_memory_mut;
use crate::ui::app::{Action, ExportOverlayData, LaReviewApp, ReviewAction};
use crate::ui::components::pills::pill_action_button;
use crate::ui::theme::{Theme, current_theme};
use crate::ui::{icons, spacing, typography};
use eframe::egui;
use egui::Margin;

pub fn render(ctx: &egui::Context, app: &mut LaReviewApp, data: &ExportOverlayData) {
    let mut open = true;
    let viewport_rect = ctx.input(|i| i.viewport().inner_rect).unwrap_or_else(|| {
        let rect = ctx.available_rect();
        egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), rect.size())
    });

    if viewport_rect.width() < 100.0 || viewport_rect.height() < 100.0 {
        return;
    }

    let theme = current_theme();
    let review_title = app
        .state
        .ui
        .selected_review_id
        .as_ref()
        .and_then(|id| app.state.domain.reviews.iter().find(|r| &r.id == id))
        .map(|r| r.title.clone())
        .unwrap_or_else(|| "Review".to_string());

    egui::Window::new("Export Review Window")
        .id(egui::Id::new("export_review_overlay"))
        .open(&mut open)
        .fixed_rect(viewport_rect)
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .frame(egui::Frame::window(&ctx.style()).inner_margin(0.0))
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                // Header
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
                                    app.dispatch(Action::Review(ReviewAction::CloseExportPreview));
                                }
                            });
                        });
                    });

                ui.separator();

                let footer_height = 60.0;
                let available_height = ui.available_height() - footer_height - 1.0;

                // Main Body
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), available_height),
                    egui::Layout::left_to_right(egui::Align::Min),
                    |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                        let mut sidebar_width = with_ui_memory_mut(ui.ctx(), |mem| {
                            if mem.export.sidebar_width > 50.0 {
                                mem.export.sidebar_width
                            } else {
                                300.0
                            }
                        });

                        // Sidebar
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
                                            ui.add_space(spacing::SPACING_SM);

                                            let id = ui.make_persistent_id("export_options_collapsed");
                                            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true).show_header(ui, |ui| {
                                                ui.label(typography::bold("Options"));
                                            }).body(|ui| {
                                                let mut options = data.options.clone();
                                                let mut changed = false;

                                                fn icon_checkbox(ui: &mut egui::Ui, theme: &Theme, value: &mut bool, label: &str, changed: &mut bool) {
                                                    ui.horizontal(|ui| {
                                                        let icon = if *value { icons::ICON_CHECK_SQUARE } else { icons::ICON_SQUARE };
                                                        let resp = ui.label(typography::body(format!("{} {}", icon, label)).color(theme.text_primary))
                                                            .interact(egui::Sense::click());
                                                        if resp.clicked() {
                                                            *value = !*value;
                                                            *changed = true;
                                                        }
                                                    });
                                                }

                                                ui.add_space(spacing::SPACING_XS);
                                                icon_checkbox(ui, &theme, &mut options.include_summary, "Include Summary", &mut changed);
                                                icon_checkbox(ui, &theme, &mut options.include_stats, "Include Stats", &mut changed);
                                                icon_checkbox(ui, &theme, &mut options.include_metadata, "Include Metadata", &mut changed);
                                                icon_checkbox(ui, &theme, &mut options.include_tasks, "Include Tasks", &mut changed);
                                                icon_checkbox(ui, &theme, &mut options.include_feedbacks, "Include Feedbacks", &mut changed);
                                                ui.add_space(spacing::SPACING_SM);

                                                if changed {
                                                    app.dispatch(Action::Review(ReviewAction::UpdateExportOptions(options)));
                                                }
                                            });

                                            ui.separator();

                                            ui.add_space(spacing::SPACING_SM);
                                            ui.horizontal(|ui| {
                                                ui.add_space(spacing::SPACING_MD);
                                                ui.label(typography::bold("Feedback"));
                                            });
                                            ui.add_space(spacing::SPACING_SM);

                                            ui.horizontal(|ui| {
                                                ui.add_space(spacing::SPACING_MD);
                                                if pill_action_button(ui, icons::ICON_CHECK, "Select All", true, theme.border).clicked() {
                                                    app.dispatch(Action::Review(ReviewAction::SelectAllExportFeedbacks));
                                                }
                                                ui.add_space(spacing::SPACING_XS);
                                                if pill_action_button(ui, icons::ACTION_CLOSE, "Clear", true, theme.border).clicked() {
                                                    app.dispatch(Action::Review(ReviewAction::ClearExportFeedbacks));
                                                }
                                            });
                                            ui.add_space(spacing::SPACING_SM);

                                            let review_feedbacks = app.state.domain.feedbacks.clone();

                                            ui.scope(|ui| {
                                                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                                                if let Some(action) = crate::ui::views::review::feedback_list::render_feedback_list(
                                                    ui,
                                                    &review_feedbacks,
                                                    None,
                                                    true,
                                                    &data.options.selected_feedback_ids,
                                                    false,
                                                    &theme,
                                                ) {
                                                    app.dispatch(Action::Review(action));
                                                }
                                            });
                                        });
                                }
                            );
                        });

                        // Resize Handle
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
                            with_ui_memory_mut(ui.ctx(), |mem| mem.export.sidebar_width = sidebar_width);
                        }

                        if resp.hovered() || resp.dragged() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                        }

                        // Preview Area
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
                                                    if data.is_exporting {
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
                                                    } else if let Some(p) = data.preview.as_ref() {
                                                        crate::ui::components::render_markdown(ui, p);
                                                    }
                                                });
                                        });
                                    });
                            });
                        });
                    },
                );

                ui.separator();

                // Footer
                egui::Frame::NONE
                    .inner_margin(Margin {
                        left: spacing::SPACING_MD as i8,
                        right: spacing::SPACING_MD as i8,
                        top: spacing::SPACING_SM as i8,
                        bottom: spacing::SPACING_SM as i8,
                    })
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(spacing::SPACING_XS);

                            if pill_action_button(ui, icons::ACTION_CLOSE, "Cancel", true, theme.border).clicked() {
                                app.dispatch(Action::Review(ReviewAction::CloseExportPreview));
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let save_text = if app.state.ui.export_save_success && app.state.ui.export_save_shown_frames < 180 {
                                    "Saved!"
                                } else {
                                    "Save Review..."
                                };

                                if pill_action_button(ui, icons::ACTION_SAVE, save_text, true, theme.brand).clicked()
                                    && let Some(path) = rfd::FileDialog::new()
                                        .add_filter("Markdown", &["md"])
                                        .set_file_name("review_export.md")
                                        .save_file() {
                                    app.dispatch(Action::Review(ReviewAction::ExportReviewToFile { path }));
                                    app.dispatch(Action::Review(ReviewAction::ResetExportSaveSuccess));
                                }

                                ui.add_space(spacing::SPACING_XS);

                                if pill_action_button(ui, icons::ACTION_COPY, "Copy", true, theme.brand).clicked()
                                    && let Some(preview) = data.preview.as_ref() {
                                    ui.ctx().output_mut(|o| o.commands.push(egui::OutputCommand::CopyText(preview.clone())));
                                    app.dispatch(Action::Review(ReviewAction::ResetExportCopySuccess));
                                }
                            });
                        });
                    });
            });
        });

    if !open {
        app.dispatch(Action::Review(ReviewAction::CloseExportPreview));
    }
}
