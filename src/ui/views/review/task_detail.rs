use super::format_timestamp;
use crate::ui::app::{Action, FullDiffView, LaReviewApp, ReviewAction};
use crate::ui::components::DiffAction;
use crate::ui::spacing::{self, SPACING_XL};
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
    fn render_description_tab(&mut self, ui: &mut egui::Ui, task: &crate::domain::ReviewTask) {
        egui::Frame::NONE
            .inner_margin(spacing::SPACING_XL)
            .show(ui, |ui| {
                let max_width = 720.0;
                let diff_width = ui.available_width() - max_width;
                ui.vertical(|ui| {
                    // Maximized paragraph spacing for "airy" feel
                    ui.spacing_mut().item_spacing.y = 28.0;

                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.set_max_width(max_width);
                            // Description
                            let description = crate::infra::normalize_newlines(&task.description);

                            // Balanced Typography for better readability and alignment
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

                                // "Airy" but connected spacing for blocks
                                ui.spacing_mut().item_spacing.y = 12.0;
                                ui.spacing_mut().indent = 16.0;

                                // Theme-integrated colors for markdown elements
                                ui.visuals_mut().override_text_color =
                                    Some(current_theme().text_secondary);
                                ui.visuals_mut().widgets.noninteractive.fg_stroke.color =
                                    current_theme().text_secondary;
                                ui.visuals_mut().extreme_bg_color = current_theme().bg_tertiary; // Code blocks
                                ui.visuals_mut().widgets.noninteractive.bg_fill =
                                    current_theme().bg_tertiary; // Other elements

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

                                // Insight (if any)
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
                                                // Also larger font for insight
                                                ui.scope(|ui| {
                                                    ui.visuals_mut().override_text_color =
                                                        Some(current_theme().text_primary);
                                                    ui.visuals_mut()
                                                        .widgets
                                                        .noninteractive
                                                        .fg_stroke
                                                        .color = current_theme().text_primary;
                                                    ui.visuals_mut().extreme_bg_color =
                                                        current_theme().bg_surface; // Slightly different for contrast

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

    fn render_diagram_tab(&mut self, ui: &mut egui::Ui, task: &crate::domain::ReviewTask) {
        egui::Frame::NONE
            .inner_margin(spacing::SPACING_XL)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_min_height(400.0);
                    let go_to_settings = crate::ui::components::diagram::diagram_view(
                        ui,
                        &task.diagram,
                        ui.visuals().dark_mode,
                    );
                    if go_to_settings {
                        self.switch_to_settings();
                    }
                });
            });
    }

    fn render_changes_tab(&mut self, ui: &mut egui::Ui, task: &crate::domain::ReviewTask) {
        // Build unified diff from diff_refs instead of stored diffs
        let unified_diff = match &self.state.cached_unified_diff {
            Some((cached_diff_refs, diff_string)) if cached_diff_refs == &task.diff_refs => {
                // Cache Hit: Diff refs haven't changed, use the cached string
                diff_string.clone()
            }
            _ => {
                // Cache Miss: Recalculate and update cache using diff_refs
                let new_diff = if !task.diff_refs.is_empty() {
                    // Look up the canonical run diff text using the task.run_id
                    let run = self.state.runs.iter().find(|r| r.id == task.run_id);
                    match run {
                        Some(run) => match crate::infra::diff_index::DiffIndex::new(&run.diff_text)
                        {
                            Ok(diff_index) => match diff_index.render_unified_diff(&task.diff_refs)
                            {
                                Ok((diff_text, _ordered_files)) => diff_text,
                                Err(_) => String::new(),
                            },
                            Err(_) => String::new(),
                        },
                        None => String::new(),
                    }
                } else {
                    String::new()
                };

                // Update the cache with the new diff and the current diff_refs as the key
                self.state.cached_unified_diff = Some((task.diff_refs.clone(), new_diff.clone()));

                new_diff
            }
        };

        // Determine if the current task has an active line note (for highlighting)
        // REMOVED: Frame wrapper for simpler look
        egui::Frame::NONE
            .inner_margin(spacing::SPACING_XL)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_min_height(300.0);

                    ui.push_id(("unified_diff", &task.id), |ui| {
                        // RENDER DIFF WITH INLINE COMMENT FUNCTIONALITY
                        let action =
                            crate::ui::components::render_diff_editor_with_comment_callback(
                                ui,
                                &unified_diff,
                                "diff",
                                true,
                                None,
                                None,
                            );

                        match action {
                            DiffAction::OpenFullWindow => {
                                self.dispatch(Action::Review(ReviewAction::OpenFullDiff(
                                    FullDiffView {
                                        title: format!("Task diff - {}", task.title),
                                        text: unified_diff.clone(),
                                    },
                                )));
                            }
                            DiffAction::AddNote {
                                line_number,
                                file_path,
                                ..
                            } => {
                                self.dispatch(Action::Review(ReviewAction::OpenThread {
                                    task_id: task.id.clone(),
                                    thread_id: None,
                                    file_path: Some(file_path),
                                    line_number: Some(line_number as u32),
                                }));
                            }
                            DiffAction::ViewNotes {
                                file_path,
                                line_number,
                            } => {
                                let thread_id = self
                                    .state
                                    .threads
                                    .iter()
                                    .find(|thread| {
                                        thread.task_id.as_ref() == Some(&task.id)
                                            && thread
                                                .anchor
                                                .as_ref()
                                                .and_then(|a| a.file_path.as_ref())
                                                == Some(&file_path)
                                            && thread.anchor.as_ref().and_then(|a| a.line_number)
                                                == Some(line_number)
                                    })
                                    .map(|thread| thread.id.clone());
                                self.dispatch(Action::Review(ReviewAction::OpenThread {
                                    task_id: task.id.clone(),
                                    thread_id,
                                    file_path: Some(file_path),
                                    line_number: Some(line_number),
                                }));
                            }
                            _ => {}
                        }
                    });
                });
            });
    }

    fn render_discussion_tab(&mut self, ui: &mut egui::Ui, task: &crate::domain::ReviewTask) {
        // Check for active thread first (Inline Thread View)
        if let Some(thread_ctx) = &self.state.active_thread {
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
            b.updated_at
                .cmp(&a.updated_at)
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

            let status_v = super::visuals::status_visuals(thread.status, &theme);
            let impact_v = super::visuals::impact_visuals(thread.impact, &theme);

            let comments = self.state.thread_comments.get(&thread.id);
            let reply_count = comments.map(|items| items.len()).unwrap_or(0);
            let updated_label = format_timestamp(&thread.updated_at);

            // 1. Reserve a spot in the painter command list *before* drawing text.
            //    This ensures the background renders BEHIND the text.
            let bg_shape_idx = ui.painter().add(egui::Shape::Noop);

            // 2. Render the row content inside a Frame
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

                            // Status
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

                            // Dot Separator
                            ui.label(egui::RichText::new("·").color(theme.text_muted).size(12.0));

                            // Impact
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

                            // Dot Separator
                            ui.label(egui::RichText::new("·").color(theme.text_muted).size(12.0));

                            // Comments & Path
                            let metadata = if path.is_empty() {
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

            // 3. Handle Interaction and Paint Background
            let row_rect = inner_response.response.rect;
            let row_id = ui.id().with(("thread_row", &thread.id));

            // Create an interaction area over the entire drawn row
            let response = ui.interact(row_rect, row_id, egui::Sense::click());

            if response.hovered() {
                // Replace the 'Noop' shape we reserved earlier with the actual background
                ui.painter().set(
                    bg_shape_idx,
                    egui::Shape::rect_filled(
                        row_rect,
                        crate::ui::spacing::RADIUS_MD, // Apply consistent rounding
                        theme.bg_secondary,            // The requested hover color
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
