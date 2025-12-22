use crate::ui::app::{Action, LaReviewApp, ReviewAction};
use crate::ui::spacing;
use crate::ui::theme::current_theme;
use crate::ui::views::review::format_timestamp;
use eframe::egui;
use egui_phosphor::regular as icons;

impl LaReviewApp {
    pub(crate) fn render_discussion_tab(
        &mut self,
        ui: &mut egui::Ui,
        task: &crate::domain::ReviewTask,
    ) {
        if ui.available_width() < 50.0 {
            return;
        }

        if let Some(thread_ctx) = &self.state.ui.active_thread {
            let view = crate::ui::views::review::thread_detail::ThreadDetailView {
                task_id: task.id.clone(),
                thread_id: thread_ctx.thread_id.clone(),
                file_path: thread_ctx.file_path.clone(),
                line_number: thread_ctx.line_number,
            };
            self.render_thread_detail(ui, &view);
            return;
        }

        let theme = current_theme();
        let mut task_threads: Vec<crate::domain::Thread> = self
            .state
            .domain
            .threads
            .iter()
            .filter(|thread| thread.task_id.as_ref() == Some(&task.id))
            .cloned()
            .collect();

        if task_threads.is_empty() {
            egui::Frame::NONE
                .inner_margin(spacing::SPACING_XL)
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.label(
                            egui::RichText::new(icons::CHAT_CIRCLE)
                                .size(44.0)
                                .color(theme.text_disabled),
                        );
                        ui.add_space(spacing::SPACING_MD);
                        ui.heading("No discussions yet");
                        ui.label(
                            egui::RichText::new(
                                "Add comments in the 'Changes' tab or start a general thread.",
                            )
                            .color(theme.text_muted),
                        );
                    });
                });
            return;
        }

        task_threads.sort_by(|a, b| {
            a.status
                .rank()
                .cmp(&b.status.rank())
                .then_with(|| b.updated_at.cmp(&a.updated_at))
                .then_with(|| b.created_at.cmp(&a.created_at))
        });

        for (index, thread) in task_threads.iter().enumerate() {
            let (path, line) = thread
                .anchor
                .as_ref()
                .map(|a| {
                    (
                        a.file_path.clone().unwrap_or_default(),
                        a.line_number.unwrap_or(0),
                    )
                })
                .unwrap_or_default();

            let title = if thread.title.is_empty() {
                "Untitled thread".to_string()
            } else {
                thread.title.clone()
            };

            let status_v = crate::ui::views::review::visuals::status_visuals(thread.status, &theme);
            let impact_v = crate::ui::views::review::visuals::impact_visuals(thread.impact, &theme);

            let comments = self.state.domain.thread_comments.get(&thread.id);
            let reply_count = comments
                .map(|items: &Vec<crate::domain::Comment>| items.len())
                .unwrap_or(0);
            let updated_label = format_timestamp(&thread.updated_at);

            let bg_shape_idx = ui.painter().add(egui::Shape::Noop);

            let inner_response = egui::Frame::NONE
                .inner_margin(egui::Margin::symmetric(
                    spacing::SPACING_XL as i8,
                    spacing::SPACING_MD as i8,
                ))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&title)
                                    .strong()
                                    .color(theme.text_primary),
                            );

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        egui::RichText::new(updated_label)
                                            .color(theme.text_muted)
                                            .size(11.0),
                                    );
                                },
                            );
                        });

                        ui.add_space(spacing::SPACING_XS);

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = spacing::SPACING_SM;

                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                ui.label(
                                    egui::RichText::new(status_v.icon)
                                        .color(status_v.color)
                                        .size(12.0),
                                );
                                ui.label(
                                    egui::RichText::new(status_v.label)
                                        .color(theme.text_secondary)
                                        .size(12.0),
                                );
                            });

                            ui.label(egui::RichText::new("·").color(theme.text_muted).size(12.0));

                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                ui.label(
                                    egui::RichText::new(impact_v.icon)
                                        .color(impact_v.color)
                                        .size(12.0),
                                );
                                ui.label(
                                    egui::RichText::new(impact_v.label)
                                        .color(theme.text_secondary)
                                        .size(12.0),
                                );
                            });

                            ui.label(egui::RichText::new("·").color(theme.text_muted).size(12.0));

                            let metadata: String = if path.is_empty() {
                                format!("{} comments", reply_count)
                            } else {
                                format!("{} comments • {}", reply_count, path)
                            };

                            ui.label(
                                egui::RichText::new(metadata)
                                    .color(theme.text_secondary)
                                    .size(12.0),
                            );
                        });
                    })
                });

            let row_rect = inner_response.response.rect;
            let row_id = ui.id().with(("thread_row", &thread.id));
            let response = ui.interact(row_rect, row_id, egui::Sense::click());

            if response.hovered() {
                ui.painter().set(
                    bg_shape_idx,
                    egui::Shape::rect_filled(
                        row_rect,
                        crate::ui::spacing::RADIUS_MD,
                        theme.bg_secondary,
                    ),
                );
            }

            let row_response = ui.interact(
                response.rect,
                ui.id().with(("thread_row", &thread.id)),
                egui::Sense::click(),
            );
            let row_response = row_response.on_hover_cursor(egui::CursorIcon::PointingHand);

            if row_response.clicked() {
                self.dispatch(Action::Review(ReviewAction::OpenThread {
                    task_id: task.id.clone(),
                    thread_id: Some(thread.id.clone()),
                    file_path: if path.is_empty() {
                        None
                    } else {
                        Some(path.clone())
                    },
                    line_number: if line == 0 { None } else { Some(line) },
                }));
            }

            if index + 1 < task_threads.len() {
                ui.separator();
            }
        }
    }
}
