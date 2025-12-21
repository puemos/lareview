use crate::ui::app::{Action, LaReviewApp, ReviewAction};
use crate::ui::spacing;
use crate::ui::theme::current_theme;
use eframe::egui;
use egui_phosphor::regular as icons;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
enum ReviewTab {
    Description,
    Diagram,
    Changes,
    Discussion,
}

impl LaReviewApp {
    /// Renders the detailed view of the selected task
    pub(super) fn render_task_detail(
        &mut self,
        ui: &mut egui::Ui,
        task: &crate::domain::ReviewTask,
    ) {
        // Safety: ensure enough width for margins
        let min_width = spacing::SPACING_XL * 2.0 + 10.0;
        if ui.available_width() < min_width {
            return;
        }

        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(
                spacing::SPACING_XL as i8,
                spacing::SPACING_XS as i8,
            ))
            .show(ui, |ui| {
                // 1. Task Header (Title only for balanced wrapping)
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&task.title)
                            .size(22.0)
                            .color(current_theme().text_primary),
                    )
                    .wrap(),
                );

                ui.add_space(spacing::SPACING_SM);

                // 2. Metadata row (Status + Risk + Stats)
                let mut status_changed = false;
                let row_height = 28.0;
                let status_width = 140.0;

                let status_visuals = |status: crate::domain::TaskStatus| match status {
                    crate::domain::TaskStatus::Pending => {
                        (icons::CIRCLE, "To do", current_theme().brand)
                    }
                    crate::domain::TaskStatus::InProgress => {
                        (icons::CIRCLE_HALF, "In progress", current_theme().accent)
                    }
                    crate::domain::TaskStatus::Done => {
                        (icons::CHECK_CIRCLE, "Done", current_theme().success)
                    }
                    crate::domain::TaskStatus::Ignored => {
                        (icons::X_CIRCLE, "Ignored", current_theme().destructive)
                    }
                };

                let status_widget_text =
                    |icon: &str,
                     icon_color: egui::Color32,
                     label: &str,
                     label_color: egui::Color32| {
                        let mut job = egui::text::LayoutJob::default();
                        let icon_format = egui::text::TextFormat {
                            font_id: egui::FontId::proportional(12.0),
                            color: icon_color,
                            ..Default::default()
                        };
                        let label_format = egui::text::TextFormat {
                            font_id: egui::FontId::proportional(12.0),
                            color: label_color,
                            ..Default::default()
                        };
                        job.append(icon, 0.0, icon_format);
                        job.append(label, 6.0, label_format);
                        egui::WidgetText::from(job)
                    };

                ui.scope(|ui| {
                    let old_interact_size = ui.spacing().interact_size;
                    ui.spacing_mut().interact_size.y = row_height;

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = spacing::SPACING_SM;

                        // Status Dropdown
                        let (selected_icon, selected_label, selected_color) =
                            status_visuals(task.status);
                        let selected_text = status_widget_text(
                            selected_icon,
                            selected_color,
                            selected_label,
                            current_theme().text_primary,
                        );

                        egui::ComboBox::from_id_salt(ui.id().with(("task_status", &task.id)))
                            .selected_text(selected_text)
                            .width(status_width)
                            .show_ui(ui, |ui| {
                                let mut next_status: Option<crate::domain::TaskStatus> = None;

                                for status in [
                                    crate::domain::TaskStatus::Pending,
                                    crate::domain::TaskStatus::InProgress,
                                    crate::domain::TaskStatus::Done,
                                    crate::domain::TaskStatus::Ignored,
                                ] {
                                    let (icon, label, color) = status_visuals(status);
                                    let text = status_widget_text(
                                        icon,
                                        color,
                                        label,
                                        current_theme().text_primary,
                                    );
                                    let selected = task.status == status;
                                    if ui.selectable_label(selected, text).clicked() {
                                        next_status = Some(status);
                                    }
                                }

                                if let Some(next_status) = next_status
                                    && next_status != task.status
                                {
                                    self.set_task_status(&task.id, next_status);
                                    status_changed = true;
                                }
                            });

                        // Dot Separator
                        ui.add_space(spacing::SPACING_XS);
                        ui.label(
                            egui::RichText::new("·")
                                .color(current_theme().text_muted)
                                .size(14.0),
                        );
                        ui.add_space(spacing::SPACING_XS);

                        // Risk Indicator
                        let (risk_icon, risk_fg, risk_label) = match task.stats.risk {
                            crate::domain::RiskLevel::High => (
                                icons::CARET_CIRCLE_DOUBLE_UP,
                                current_theme().destructive,
                                "High risk",
                            ),
                            crate::domain::RiskLevel::Medium => {
                                (icons::CARET_CIRCLE_UP, current_theme().warning, "Med risk")
                            }
                            crate::domain::RiskLevel::Low => {
                                (icons::CARET_CIRCLE_DOWN, current_theme().accent, "Low risk")
                            }
                        };

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            ui.label(egui::RichText::new(risk_icon).color(risk_fg).size(14.0));
                            ui.label(
                                egui::RichText::new(risk_label)
                                    .color(current_theme().text_muted)
                                    .size(12.0),
                            );
                        });

                        // Dot Separator
                        ui.add_space(spacing::SPACING_XS);
                        ui.label(
                            egui::RichText::new("·")
                                .color(current_theme().text_muted)
                                .size(14.0),
                        );
                        ui.add_space(spacing::SPACING_XS);

                        // Stats
                        ui.label(
                            egui::RichText::new(format!("{} files", task.files.len()))
                                .color(current_theme().text_muted)
                                .size(12.0),
                        );

                        ui.label(
                            egui::RichText::new("|")
                                .color(current_theme().text_disabled)
                                .size(12.0),
                        );

                        ui.label(
                            egui::RichText::new(format!("+{}", task.stats.additions))
                                .color(current_theme().success)
                                .size(12.0),
                        );

                        ui.label(
                            egui::RichText::new(format!("-{}", task.stats.deletions))
                                .color(current_theme().destructive)
                                .size(12.0),
                        );

                        ui.label(
                            egui::RichText::new("lines")
                                .color(current_theme().text_muted)
                                .size(12.0),
                        );
                    });

                    ui.spacing_mut().interact_size = old_interact_size;
                });

                if status_changed {
                    return;
                }

                ui.add_space(spacing::SPACING_LG);

                // 3. Tab Bar
                let mut active_tab = ui
                    .ctx()
                    .data(|d| d.get_temp::<ReviewTab>(egui::Id::new(("active_tab", &task.id))))
                    .unwrap_or(ReviewTab::Description);

                // Force Discussion tab if thread is active
                if self.state.active_thread.is_some() {
                    active_tab = ReviewTab::Discussion;
                }

                let note_count = self
                    .state
                    .threads
                    .iter()
                    .filter(|thread| thread.task_id.as_ref() == Some(&task.id))
                    .count();
                let discussion_label = if note_count > 0 {
                    format!("Discussion ({})", note_count)
                } else {
                    "Discussion".to_string()
                };

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = spacing::SPACING_MD;

                    let mut tab_button =
                        |ui: &mut egui::Ui, tab: ReviewTab, label: &str, icon: &str| {
                            let is_selected = active_tab == tab;
                            let text = format!("{} {}", icon, label);

                            let mut text = egui::RichText::new(text).size(13.0);
                            if is_selected {
                                text = text.strong().color(current_theme().brand);
                            } else {
                                text = text.color(current_theme().text_muted);
                            };

                            let resp = ui.add(
                                egui::Button::new(text)
                                    .fill(if is_selected {
                                        current_theme().brand.gamma_multiply(0.1)
                                    } else {
                                        current_theme().transparent
                                    })
                                    .stroke(egui::Stroke::NONE)
                                    .corner_radius(crate::ui::spacing::RADIUS_MD),
                            );
                            let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);

                            if resp.clicked() {
                                if self.state.active_thread.is_some() {
                                    self.dispatch(Action::Review(ReviewAction::CloseThread));
                                }
                                active_tab = tab;
                                ui.ctx().data_mut(|d| {
                                    d.insert_temp(egui::Id::new(("active_tab", &task.id)), tab)
                                });
                            }
                        };

                    tab_button(ui, ReviewTab::Description, "Description", icons::FILE_TEXT);
                    if task.diagram.as_ref().is_some_and(|d| !d.is_empty()) {
                        tab_button(ui, ReviewTab::Diagram, "Diagram", icons::CHART_BAR);
                    }
                    if !task.diff_refs.is_empty() {
                        tab_button(ui, ReviewTab::Changes, "Changes", icons::GIT_DIFF);
                    }

                    tab_button(
                        ui,
                        ReviewTab::Discussion,
                        &discussion_label,
                        icons::CHAT_CIRCLE,
                    );
                });
            }); // End of Header Frame

        ui.separator();

        // 4. Content Area
        egui::ScrollArea::vertical()
            .id_salt(format!("detail_scroll_{:?}", "active_tab_placeholder")) // Note: active_tab was local, need to fetch again or move scrollarea
            .show(ui, |ui| {
                // Fetch active tab again since we are out of the closure
                let mut active_tab = ui
                    .ctx()
                    .data(|d| d.get_temp::<ReviewTab>(egui::Id::new(("active_tab", &task.id))))
                    .unwrap_or(ReviewTab::Description);
                if self.state.active_thread.is_some() {
                    active_tab = ReviewTab::Discussion;
                }

                match active_tab {
                    ReviewTab::Description => self.render_description_tab(ui, task),

                    ReviewTab::Diagram => self.render_diagram_tab(ui, task),

                    ReviewTab::Changes => self.render_changes_tab(ui, task),

                    ReviewTab::Discussion => self.render_discussion_tab(ui, task),
                }
            });
    }
}
