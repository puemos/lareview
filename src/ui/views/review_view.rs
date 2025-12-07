//! Review view (egui version)
use crate::ui::app::LaReviewApp;
use eframe::egui;
// diff editor helper
use crate::ui::components::diff::render_diff_editor;
use catppuccin_egui::MOCHA;

impl LaReviewApp {
    pub fn ui_review(&mut self, ui: &mut egui::Ui) {
        ui.heading("Review tasks");

        // show error if any
        if let Some(err) = &self.state.review_error {
            ui.colored_label(MOCHA.red, err); // Use MOCHA.red for error
        }

        ui.separator();

        // Three-column layout for PRs, Tasks, and Task Details
        ui.columns(3, |columns| {
            // Column 1: PR List
            columns[0].vertical(|ui| {
                ui.heading(egui::RichText::new("Pull Requests").color(MOCHA.text));
                ui.separator();

                // Option to view all PRs
                let all_prs_selected = self.state.selected_pr_id.is_none();
                if ui.selectable_label(all_prs_selected, "◆ All PRs").clicked() {
                    self.state.selected_pr_id = None;
                    self.sync_review_from_db();
                }

                // Store the PR ID if clicked, then update after the loop to avoid borrow conflicts
                let mut pr_to_select = None;
                for pr in &self.state.prs {
                    let selected = self.state.selected_pr_id.as_ref() == Some(&pr.id);
                    if ui.selectable_label(selected, &pr.title).clicked() {
                        pr_to_select = Some(pr.id.clone());
                    }
                }
                if let Some(pr_id) = pr_to_select {
                    self.state.selected_pr_id = Some(pr_id);
                    self.sync_review_from_db();
                }
            });

            // Column 2: Tasks within selected PR
            columns[1].vertical(|ui| {
                // Use the getter for filtered tasks
                let current_tasks = self.state.tasks();
                ui.heading(format!("Tasks ({})", current_tasks.len()));
                ui.separator();

                for task in &current_tasks {
                    // Iterate over filtered tasks
                    ui.horizontal(|ui| {
                        let selected = Some(task.id.clone()) == self.state.selected_task_id;
                        let response = ui.selectable_label(selected, &task.title);

                        if response.clicked() {
                            self.state.selected_task_id = Some(task.id.clone());
                            if let Some(note) = self.note_repo.find_by_task(&task.id).ok().flatten()
                            {
                                self.state.current_note = Some(note.body);
                            } else {
                                self.state.current_note = Some(String::new());
                            }
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let delete_response = ui.add(
                                egui::Button::new(egui::RichText::new("🗑 DELETE").color(MOCHA.red))
                                    .frame(false),
                            );
                            if delete_response.clicked() {
                                self.delete_review_task(&task.id);
                            }
                        });
                    });
                }
            });

            // Column 3: Main task view
            columns[2].vertical(|ui| {
                // The PR info will come from self.state.pr_title etc. which are now set by sync_review_from_db
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

                if let Some(task_id) = &self.state.selected_task_id {
                    // Clone the task data we need to avoid borrow issues
                    let task_data = self
                        .state
                        .tasks()
                        .iter()
                        .find(|t| &t.id == task_id)
                        .map(|t| {
                            // Use self.state.tasks()
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
                                    let hunk = patch.hunk.clone();
                                    render_diff_editor(ui, &hunk, "diff");
                                });
                            }
                        }
                    } else {
                        ui.label("Task not found");
                    }
                } else {
                    ui.label("No task selected");
                }
                // Save after the UI is done to avoid borrow conflicts
                if should_save {
                    self.save_current_note();
                }
            });
        }); // Closing brace for ui.columns(3, |columns| { ... });
    }
}
