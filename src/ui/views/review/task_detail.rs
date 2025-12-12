use crate::infra::diff::{combine_diffs_to_unified_diff, extract_file_path_from_diff};
use crate::ui::app::LaReviewApp;
use crate::ui::components::badge::badge;
use crate::ui::components::pills::pill_divider;
use crate::ui::components::task_status_chip::task_status_chip;
use crate::ui::components::{DiffAction, LineContext};
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
        egui::ScrollArea::vertical()
            .id_salt("detail_scroll")
            .show(ui, |ui| {
                // 1. Task Header
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&task.title)
                            .size(24.0)
                            .color(MOCHA.text),
                    )
                    .wrap(),
                );

                ui.add_space(8.0);

                // 2. Metadata Badges (includes status actions)
                let mut status_changed = false;
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);

                    let to_do_resp = task_status_chip(
                        ui,
                        icons::CIRCLE,
                        "To do",
                        task.status == crate::domain::TaskStatus::Pending,
                        MOCHA.mauve, // gray
                    );
                    if to_do_resp.clicked() && task.status != crate::domain::TaskStatus::Pending {
                        self.set_task_status(&task.id, crate::domain::TaskStatus::Pending);
                        status_changed = true;
                    }

                    let in_progress_resp = task_status_chip(
                        ui,
                        icons::CIRCLE_HALF,
                        "In progress",
                        task.status == crate::domain::TaskStatus::InProgress,
                        MOCHA.blue,
                    );
                    if in_progress_resp.clicked()
                        && task.status != crate::domain::TaskStatus::InProgress
                    {
                        self.set_task_status(&task.id, crate::domain::TaskStatus::InProgress);
                        status_changed = true;
                    }

                    let done_resp = task_status_chip(
                        ui,
                        icons::CHECK_CIRCLE,
                        "Done",
                        task.status == crate::domain::TaskStatus::Done,
                        MOCHA.green,
                    );
                    if done_resp.clicked() && task.status != crate::domain::TaskStatus::Done {
                        self.set_task_status(&task.id, crate::domain::TaskStatus::Done);
                        status_changed = true;
                    }

                    let ignored_resp = task_status_chip(
                        ui,
                        icons::X_CIRCLE,
                        "Ignored",
                        task.status == crate::domain::TaskStatus::Ignored,
                        MOCHA.red,
                    );
                    if ignored_resp.clicked() && task.status != crate::domain::TaskStatus::Ignored {
                        self.set_task_status(&task.id, crate::domain::TaskStatus::Ignored);
                        status_changed = true;
                    }

                    pill_divider(ui);

                    let (risk_icon, risk_fg, risk_text) = match task.stats.risk {
                        crate::domain::RiskLevel::High => {
                            (icons::CARET_CIRCLE_DOUBLE_UP, MOCHA.red, "High risk")
                        }
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
                        "{} files |+{} -{} lines",
                        task.files.len(),
                        task.stats.additions,
                        task.stats.deletions
                    );
                    badge(ui, &stats_text, MOCHA.surface0, MOCHA.subtext0);
                });

                if status_changed {
                    return;
                }

                ui.add_space(16.0);

                // 3. Context Section (Description + Insight)
                egui::Frame::group(ui.style())
                    .fill(MOCHA.surface0.gamma_multiply(0.3))
                    .stroke(egui::Stroke::new(1.0, MOCHA.surface1))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());

                        // Description
                        ui.label(
                            egui::RichText::new("Description")
                                .strong()
                                .color(MOCHA.lavender),
                        );
                        ui.label(egui::RichText::new(&task.description).color(MOCHA.text));

                        // Insight (if any)
                        if let Some(insight) = &task.insight {
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(egui_phosphor::regular::SPARKLE)
                                        .color(MOCHA.yellow),
                                );
                                ui.label(
                                    egui::RichText::new("AI Insight")
                                        .strong()
                                        .color(MOCHA.yellow),
                                );
                            });
                            ui.label(egui::RichText::new(insight).italics().color(MOCHA.subtext0));
                        }
                    });

                ui.add_space(16.0);

                // Diagram Viewer
                if task.diagram.as_ref().is_some_and(|d| !d.is_empty()) {
                    ui.label(egui::RichText::new("Diagram").strong().size(16.0));
                    egui::Frame::NONE
                        .stroke(egui::Stroke::new(1.0, MOCHA.surface1))
                        .inner_margin(12.0)
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
                    ui.add_space(16.0);
                }

                // 4. Diff Viewer
                ui.label(egui::RichText::new("Changes").strong().size(16.0));

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
                        .inner_margin(12.0)
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
                                        self.state.full_diff = Some(crate::ui::app::FullDiffView {
                                            title: format!("Task diff - {}", task.title),
                                            text: unified_diff.clone(),
                                        });
                                    }
                                    DiffAction::AddNote {
                                        file_idx,
                                        line_idx,
                                        line_number,
                                    } => {
                                        // Set the state for adding an inline note
                                        // Get the file path for the clicked line
                                        let file_path = if file_idx < task.diffs.len() {
                                            // Since diffs are strings, we need to extract the file path from the diff string
                                            extract_file_path_from_diff(&task.diffs[file_idx])
                                                .unwrap_or("unknown".to_string())
                                        } else {
                                            "unknown".to_string()
                                        };

                                        self.state.current_line_note =
                                            Some(crate::ui::app::LineNoteContext {
                                                task_id: task.id.clone(),
                                                file_idx,
                                                line_idx,
                                                line_number,
                                                file_path,
                                                note_text: String::new(), // Load existing note if one exists
                                            });

                                        // OPTIONAL: Reset main note if starting a line note
                                        self.state.current_note = None;
                                    }
                                    DiffAction::SaveNote {
                                        file_idx,
                                        line_idx: _,
                                        line_number,
                                        note_text,
                                    } => {
                                        // Handle saving the note
                                        // Get the file path for the line
                                        let file_path = if file_idx < task.diffs.len() {
                                            extract_file_path_from_diff(&task.diffs[file_idx])
                                                .unwrap_or("unknown".to_string())
                                        } else {
                                            "unknown".to_string()
                                        };

                                        // Save the line-specific note to the database
                                        let note = crate::domain::Note {
                                            task_id: task.id.clone(),
                                            body: note_text,
                                            updated_at: chrono::Utc::now().to_rfc3339(),
                                            file_path: Some(file_path),
                                            line_number: Some(line_number as u32),
                                        };

                                        if let Err(err) = self.note_repo.save(&note) {
                                            self.state.review_error =
                                                Some(format!("Failed to save line note: {}", err));
                                        } else {
                                            self.state.review_error = None;
                                        }

                                        // Clear the active comment line after saving
                                        self.state.current_line_note = None;
                                    }
                                    _ => {}
                                }
                            });

                            // We no longer show a modal window here - the comment input is now rendered inline in the diff component
                            // The state is still used to track which line should have the active comment input

                            // The inline comment input is now handled directly in the diff component
                        });
                }

                ui.add_space(16.0);

                // 5. Notes Section
                egui::CollapsingHeader::new("Review Notes")
                    .id_salt("notes_header")
                    .default_open(true)
                    .show(ui, |ui| {
                        if let Some(note_text) = &mut self.state.current_note {
                            ui.add(
                                egui::TextEdit::multiline(note_text)
                                    .id_salt(ui.id().with(("task_note", &task.id)))
                                    .hint_text("Add your review notes here...")
                                    .desired_rows(4)
                                    .desired_width(f32::INFINITY),
                            );
                            ui.add_space(4.0);
                            if ui.button("Save Note").clicked() {
                                self.save_current_note();
                            }
                        }
                    });

                ui.add_space(20.0);
            });
    }
}
