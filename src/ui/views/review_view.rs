//! Review view (egui version)
use crate::ui::app::LaReviewApp;
use eframe::egui;
// diff editor helper
use crate::ui::components::diff::render_diff_editor;

impl LaReviewApp {
    pub fn ui_review(&mut self, ui: &mut egui::Ui) {
        ui.heading("Review tasks");

        // show error if any
        if let Some(err) = &self.state.review_error {
            ui.colored_label(egui::Color32::RED, err);
        }

        ui.separator();

        ui.horizontal(|ui| {
            ui.label(format!(
                "PR: {} ({})",
                self.state.pr_title, self.state.pr_repo
            ));
            ui.label(format!("Author: {}", self.state.pr_author));
            ui.label(format!("Branch: {}", self.state.pr_branch));
        });

        ui.separator();

        // Track if save button was clicked
        let mut should_save = false;

        ui.horizontal(|ui| {
            // sidebar: tasks
            ui.vertical(|ui| {
                ui.heading(format!("Tasks ({})", self.state.tasks.len()));
                for task in &self.state.tasks {
                    let selected = Some(task.id.clone()) == self.state.selected_task_id;
                    if ui.selectable_label(selected, &task.title).clicked() {
                        self.state.selected_task_id = Some(task.id.clone());
                    }
                }
            });

            ui.separator();

            // main task view
            ui.vertical(|ui| {
                if let Some(task_id) = &self.state.selected_task_id {
                    // Clone the task data we need to avoid borrow issues
                    let task_data = self.state.tasks.iter().find(|t| &t.id == task_id).map(|t| {
                        (
                            t.title.clone(),
                            format!("{:?}", t.status),
                            format!("{:?}", t.stats.risk),
                            t.patches.clone(),
                        )
                    });

                    if let Some((title, status, risk_str, patches)) = task_data {
                        ui.heading(&title);
                        ui.label(format!("Status: {}", status));
                        ui.label(format!("Risk: {}", risk_str));

                        ui.separator();

                        // Notes
                        ui.heading("Notes");
                        if let Some(note_text) = &mut self.state.current_note {
                            ui.horizontal(|ui| {
                                if ui.button("Save note").clicked() {
                                    should_save = true;
                                }
                            });
                            ui.add(
                                egui::TextEdit::multiline(note_text)
                                    .desired_rows(6)
                                    .desired_width(f32::INFINITY),
                            );
                        } else {
                            ui.label("No note");
                        }

                        ui.separator();

                        // Patches
                        ui.heading("Patches");
                        if patches.is_empty() {
                            ui.label("No patches");
                        } else {
                            for patch in &patches {
                                ui.group(|ui| {
                                    ui.label(format!("File: {}", patch.file));
                                    let mut hunk = patch.hunk.clone();
                                    render_diff_editor(ui, &mut hunk, "diff");
                                });
                            }
                        }
                    } else {
                        ui.label("Task not found");
                    }
                } else {
                    ui.label("No task selected");
                }
            });
        });

        // Save after the UI is done to avoid borrow conflicts
        if should_save {
            self.save_current_note();
        }
    }
}
