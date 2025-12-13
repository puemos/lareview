use crate::infra::diff::{combine_diffs_to_unified_diff, extract_file_path_from_diff};
use crate::ui::app::{Action, FullDiffView, LaReviewApp, LineNoteContext, ReviewAction};
use crate::ui::components::badge::badge;
use crate::ui::components::pills::pill_divider;
use crate::ui::components::{DiffAction, LineContext};
use crate::ui::spacing;
use catppuccin_egui::MOCHA;
use eframe::egui;
use egui_phosphor::regular as icons;

impl LaReviewApp {
    /// Renders the detailed view of the selected task
    pub(super) fn render_task_detail(
        &mut self,
        ui: &mut egui::Ui,
        task: &crate::domain::ReviewTask,
    ) {
        let section_title = |text: &str| egui::RichText::new(text).strong().size(14.0);

        egui::ScrollArea::vertical()
            .id_salt("detail_scroll")
            .show(ui, |ui| {
                // 1. Task Header (title)
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&task.title)
                            .size(22.0)
                            .color(MOCHA.text),
                    )
                    .wrap(),
                );

                ui.add_space(spacing::SPACING_SM);

                // 2. Metadata row (risk + stats + status)
                let mut status_changed = false;
                let row_width = ui.available_width();
                let row_height = 28.0;
                let status_width = 160.0;
                let gap = spacing::SPACING_SM;
                let left_width = (row_width - status_width - gap).max(120.0);

                let status_visuals = |status: crate::domain::TaskStatus| match status {
                    crate::domain::TaskStatus::Pending => (icons::CIRCLE, "To do", MOCHA.mauve),
                    crate::domain::TaskStatus::InProgress => {
                        (icons::CIRCLE_HALF, "In progress", MOCHA.blue)
                    }
                    crate::domain::TaskStatus::Done => (icons::CHECK_CIRCLE, "Done", MOCHA.green),
                    crate::domain::TaskStatus::Ignored => (icons::X_CIRCLE, "Ignored", MOCHA.red),
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
                        job.append(" ", 0.0, label_format.clone());
                        job.append(label, 0.0, label_format);
                        egui::WidgetText::from(job)
                    };

                ui.scope(|ui| {
                    let old_interact_size = ui.spacing().interact_size;
                    ui.spacing_mut().interact_size.y = row_height;

                    ui.allocate_ui_with_layout(
                        egui::vec2(row_width, row_height),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.allocate_ui_with_layout(
                                egui::vec2(left_width, row_height),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.spacing_mut().item_spacing = egui::vec2(
                                        spacing::ITEM_SPACING.0,
                                        spacing::ITEM_SPACING.1,
                                    );

                                    ui.horizontal(|ui| {
                                        let (risk_icon, risk_fg, risk_text) = match task.stats.risk
                                        {
                                            crate::domain::RiskLevel::High => (
                                                icons::CARET_CIRCLE_DOUBLE_UP,
                                                MOCHA.red,
                                                "High risk",
                                            ),
                                            crate::domain::RiskLevel::Medium => {
                                                (icons::CARET_CIRCLE_UP, MOCHA.yellow, "Med risk")
                                            }
                                            crate::domain::RiskLevel::Low => {
                                                (icons::CARET_CIRCLE_DOWN, MOCHA.blue, "Low risk")
                                            }
                                        };
                                        badge(
                                            ui,
                                            &format!("{risk_icon} {risk_text}"),
                                            risk_fg.gamma_multiply(0.2),
                                            risk_fg,
                                        );

                                        pill_divider(ui);

                                        let stats_text = format!(
                                            "{} files | +{} / -{} lines",
                                            task.files.len(),
                                            task.stats.additions,
                                            task.stats.deletions
                                        );
                                        badge(ui, &stats_text, MOCHA.surface0, MOCHA.subtext0);
                                    });
                                },
                            );

                            ui.add_space(gap);
                            pill_divider(ui);
                            ui.add_space(gap);

                            ui.allocate_ui_with_layout(
                                egui::vec2(status_width, row_height),
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let (selected_icon, selected_label, selected_color) =
                                        status_visuals(task.status);
                                    let selected_text = status_widget_text(
                                        selected_icon,
                                        selected_color,
                                        selected_label,
                                        MOCHA.text,
                                    );

                                    egui::ComboBox::from_id_salt(
                                        ui.id().with(("task_status", &task.id)),
                                    )
                                    .selected_text(selected_text)
                                    .width(status_width)
                                    .show_ui(ui, |ui| {
                                        let mut next_status: Option<crate::domain::TaskStatus> =
                                            None;

                                        for status in [
                                            crate::domain::TaskStatus::Pending,
                                            crate::domain::TaskStatus::InProgress,
                                            crate::domain::TaskStatus::Done,
                                            crate::domain::TaskStatus::Ignored,
                                        ] {
                                            let (icon, label, color) = status_visuals(status);
                                            let text =
                                                status_widget_text(icon, color, label, MOCHA.text);
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
                                },
                            );
                        },
                    );

                    ui.spacing_mut().interact_size = old_interact_size;
                });

                if status_changed {
                    return;
                }

                ui.add_space(spacing::SPACING_LG);

                // 3. Context Section (Description + Insight)
                egui::Frame::group(ui.style())
                    .fill(MOCHA.surface0.gamma_multiply(0.3))
                    .stroke(egui::Stroke::new(1.0, MOCHA.surface1))
                    .inner_margin(spacing::SPACING_MD)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());

                        // Description
                        ui.label(section_title("Description").color(MOCHA.lavender));
                        ui.add_space(spacing::SPACING_XS);
                        ui.label(egui::RichText::new(&task.description).color(MOCHA.text));

                        // Insight (if any)
                        if let Some(insight) = &task.insight {
                            ui.add_space(spacing::SPACING_SM);
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(egui_phosphor::regular::SPARKLE)
                                        .color(MOCHA.yellow),
                                );
                                ui.label(section_title("AI Insight").color(MOCHA.yellow));
                            });
                            ui.add_space(spacing::SPACING_XS);
                            ui.label(egui::RichText::new(insight).italics().color(MOCHA.subtext0));
                        }
                    });

                ui.add_space(spacing::SPACING_LG);

                // Diagram Viewer
                if task.diagram.as_ref().is_some_and(|d| !d.is_empty()) {
                    ui.label(section_title("Diagram").color(MOCHA.text));
                    ui.add_space(spacing::SPACING_XS);
                    egui::Frame::NONE
                        .stroke(egui::Stroke::new(1.0, MOCHA.surface1))
                        .inner_margin(spacing::SPACING_MD)
                        .corner_radius(4.0)
                        .show(ui, |ui| {
                            ui.set_min_height(200.0);
                            let go_to_settings = crate::ui::components::diagram::diagram_view(
                                ui,
                                &task.diagram,
                                ui.visuals().dark_mode,
                            );
                            if go_to_settings {
                                self.switch_to_settings();
                            }
                        });
                    ui.add_space(spacing::SPACING_LG);
                }

                // 4. Diff Viewer
                ui.label(section_title("Changes").color(MOCHA.text));
                ui.add_space(spacing::SPACING_XS);

                let unified_diff = match &self.state.cached_unified_diff {
                    Some((cached_diffs, diff_string)) if cached_diffs == &task.diffs => {
                        // Cache Hit: Diffs haven't changed, use the cached string
                        diff_string.clone()
                    }
                    _ => {
                        // Cache Miss: Recalculate and update cache
                        let new_diff = combine_diffs_to_unified_diff(&task.diffs);

                        // Update the cache with the new diff and the current diffs as the key
                        self.state.cached_unified_diff =
                            Some((task.diffs.clone(), new_diff.clone()));

                        new_diff
                    }
                };

                // ADDED: Determine if the current task has an active line note (for highlighting)
                let active_line_context = self
                    .state
                    .current_line_note
                    .as_ref()
                    .filter(|ctx| ctx.task_id == task.id)
                    .map(|ctx| LineContext {
                        file_idx: ctx.file_idx,
                        line_idx: ctx.line_idx,
                    });

                // Determine which line has active comment input (for inline comment display)
                let _active_comment_context = self
                    .state
                    .current_line_note
                    .as_ref()
                    .filter(|ctx| ctx.task_id == task.id)
                    .map(|ctx| LineContext {
                        file_idx: ctx.file_idx,
                        line_idx: ctx.line_idx,
                    });

                if task.diffs.is_empty() {
                    ui.label(
                        egui::RichText::new("No code changes in this task (metadata only?)")
                            .italics(),
                    );
                } else {
                    egui::Frame::NONE
                        .stroke(egui::Stroke::new(1.0, MOCHA.surface1))
                        .inner_margin(spacing::SPACING_MD)
                        .corner_radius(4.0)
                        .show(ui, |ui| {
                            ui.set_min_height(300.0);

                            ui.push_id(("unified_diff", &task.id), |ui| {
                                // RENDER DIFF WITH INLINE COMMENT FUNCTIONALITY
                                let action =
                                    crate::ui::components::render_diff_editor_with_comment_callback(
                                        ui,
                                        &unified_diff,
                                        "diff",
                                        true,
                                        active_line_context, // Use for highlighting
                                        Some(&|_file_idx, _line_idx, _line_number| {
                                            // Handle the click by returning an action
                                            // The actual state change will be handled in the match block below
                                        }),
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
                                        file_idx,
                                        line_idx,
                                        line_number,
                                    } => {
                                        let file_path = if file_idx < task.diffs.len() {
                                            extract_file_path_from_diff(&task.diffs[file_idx])
                                                .unwrap_or("unknown".to_string())
                                        } else {
                                            "unknown".to_string()
                                        };

                                        self.dispatch(Action::Review(ReviewAction::StartLineNote(
                                            LineNoteContext {
                                                task_id: task.id.clone(),
                                                file_idx,
                                                line_idx,
                                                line_number,
                                                file_path,
                                                note_text: String::new(),
                                            },
                                        )));
                                    }
                                    DiffAction::SaveNote {
                                        file_idx,
                                        line_idx: _,
                                        line_number,
                                        note_text,
                                    } => {
                                        let file_path = if file_idx < task.diffs.len() {
                                            extract_file_path_from_diff(&task.diffs[file_idx])
                                                .unwrap_or("unknown".to_string())
                                        } else {
                                            "unknown".to_string()
                                        };

                                        self.dispatch(Action::Review(ReviewAction::SaveLineNote {
                                            task_id: task.id.clone(),
                                            file_path,
                                            line_number: line_number as u32,
                                            body: note_text,
                                        }));
                                    }
                                    _ => {}
                                }
                            });

                            // We no longer show a modal window here - the comment input is now rendered inline in the diff component
                            // The state is still used to track which line should have the active comment input

                            // The inline comment input is now handled directly in the diff component
                        });
                }

                ui.add_space(spacing::SPACING_LG);

                // 5. Notes Section
                egui::CollapsingHeader::new("Review Notes")
                    .id_salt("notes_header")
                    .default_open(true)
                    .show(ui, |ui| {
                        if self.state.selected_task_id.is_some() {
                            let mut note_text = self.state.current_note.clone().unwrap_or_default();

                            let response = ui.add(
                                egui::TextEdit::multiline(&mut note_text)
                                    .id_salt(ui.id().with(("task_note", &task.id)))
                                    .hint_text("Add your review notes here...")
                                    .desired_rows(4)
                                    .desired_width(f32::INFINITY),
                            );

                            if response.changed() {
                                self.dispatch(Action::Review(ReviewAction::SetCurrentNoteText(
                                    note_text.clone(),
                                )));
                            }

                            ui.add_space(spacing::SPACING_XS);
                            if ui.button("Save Note").clicked() {
                                self.dispatch(Action::Review(ReviewAction::SaveCurrentNote));
                            }
                        }
                    });

                ui.add_space(spacing::SPACING_LG);
            });
    }
}
