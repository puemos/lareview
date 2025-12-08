//! Review view (egui version) - Left-Right Tree Layout
use crate::ui::app::LaReviewApp;
use eframe::egui;
// diff editor helper
use crate::ui::components::diff::render_diff_editor;
use crate::ui::components::status::error_banner;
use catppuccin_egui::MOCHA;

impl LaReviewApp {
    pub fn ui_review(&mut self, ui: &mut egui::Ui) {
        ui.heading("Review tasks");

        // show error if any
        if let Some(err) = &self.state.review_error {
            error_banner(ui, err);
        }

        ui.separator();

        // PR Header
        ui.horizontal(|ui| {
            ui.label(format!("PR: {}", self.state.pr_title));
            ui.label(format!("({})", self.state.pr_repo));
            ui.label(format!("Author: {}", self.state.pr_author));
            ui.label(format!("Branch: {}", self.state.pr_branch));
        });

        ui.separator();

        // Group tasks by sub-flows
        let tasks_by_sub_flow = self.state.tasks_by_sub_flow();
        let all_tasks = self.state.tasks();

        // Calculate overall progress
        let total_tasks = all_tasks.len();
        let reviewed_tasks = all_tasks
            .iter()
            .filter(|t| t.status == crate::domain::TaskStatus::Reviewed)
            .count();
        let progress = if total_tasks > 0 {
            (reviewed_tasks as f32) / (total_tasks as f32)
        } else {
            0.0
        };

        // Intent Panel
        ui.group(|ui| {
            ui.heading(egui::RichText::new("INTENT").size(16.0).color(MOCHA.mauve));

            // For now, we'll use the PR title as the intent, but in the future this could be separate
            ui.label(
                egui::RichText::new(&self.state.pr_title)
                    .strong()
                    .size(14.0),
            );

            // Show overall progress
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Status: {}/{} tasks reviewed",
                    reviewed_tasks, total_tasks
                ));
                ui.label(format!("Progress: {:.0}%", progress * 100.0));
            });

            // Determine overall risk based on highest risk among tasks
            let overall_risk = if all_tasks.is_empty() {
                crate::domain::RiskLevel::Low
            } else {
                all_tasks
                    .iter()
                    .map(|t| t.stats.risk)
                    .max()
                    .unwrap_or(crate::domain::RiskLevel::Low)
            };
            ui.label(format!("Risk: {:?}", overall_risk));
        });

        ui.separator();

        // Create a stable ordering of sub-flows to prevent reordering
        let mut sub_flows: Vec<(&Option<String>, &Vec<crate::domain::ReviewTask>)> =
            tasks_by_sub_flow.iter().collect();
        sub_flows.sort_by(|(name_a, _), (name_b, _)| {
            name_a
                .as_deref()
                .unwrap_or("ZZZ")
                .cmp(name_b.as_deref().unwrap_or("ZZZ")) // Put "Uncategorized" at the end
        });

        // Use columns for a stable left-right layout
        ui.columns(2, |columns| {
            // Left column - Navigation panel
            columns[0].scope(|ui| {
                ui.set_width_range(150.0..=300.0); // Set smaller max width for tree (reduced to 300)
                ui.vertical(|ui| {
                    ui.heading("Navigation");

                    // Make navigation scrollable if it's too tall
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        // Sub-Flows Tree - Group tasks by sub-flow with stable ordering
                        for (sub_flow_name, tasks) in sub_flows {
                            let sub_flow_title =
                                sub_flow_name.as_deref().unwrap_or("Uncategorized");

                            // Calculate sub-flow progress
                            let sub_total = tasks.len();
                            let sub_reviewed = tasks
                                .iter()
                                .filter(|t| t.status == crate::domain::TaskStatus::Reviewed)
                                .count();

                            // Static header without state issues
                            ui.collapsing(
                                format!("{} ({}/{})", sub_flow_title, sub_reviewed, sub_total),
                                |ui| {
                                    // Task List within this sub-flow with risk indicator
                                    for task in tasks.iter() {
                                        let is_selected =
                                            self.state.selected_task_id.as_ref() == Some(&task.id);

                                        ui.horizontal(|ui| {
                                            // Risk indicator icon
                                            let risk_color = match task.stats.risk {
                                                crate::domain::RiskLevel::Low => MOCHA.green,
                                                crate::domain::RiskLevel::Medium => MOCHA.yellow,
                                                crate::domain::RiskLevel::High => MOCHA.red,
                                            };
                                            ui.label(
                                                egui::RichText::new(match task.stats.risk {
                                                    crate::domain::RiskLevel::Low => "🟢",
                                                    crate::domain::RiskLevel::Medium => "🟡",
                                                    crate::domain::RiskLevel::High => "🔴",
                                                })
                                                .color(risk_color),
                                            );

                                            // Use a simple selectable label for task selection
                                            if ui
                                                .selectable_label(is_selected, &task.title)
                                                .clicked()
                                            {
                                                self.state.selected_task_id = Some(task.id.clone());
                                                // Load note for the selected task
                                                if let Some(note) = self
                                                    .note_repo
                                                    .find_by_task(&task.id)
                                                    .ok()
                                                    .flatten()
                                                {
                                                    self.state.current_note = Some(note.body);
                                                } else {
                                                    self.state.current_note = Some(String::new());
                                                }
                                            }
                                        });

                                        // Add spacing between tasks
                                        ui.add_space(2.0);
                                    }
                                },
                            );
                        }
                    });
                });
            });

            // Right column - Content view
            columns[1].scope(|ui| {
                // Right panel - Content view that fills the remaining space without double borders
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Selected Task Detail Panel - no group border to avoid double border
                    if let Some(task_id) = &self.state.selected_task_id {
                        if let Some(task) = all_tasks.iter().find(|t| &t.id == task_id) {
                            ui.heading(
                                egui::RichText::new(&task.title)
                                    .size(16.0)
                                    .color(MOCHA.text),
                            );

                            // Task metadata
                            ui.horizontal(|ui| {
                                // Status indicator
                                let status_color = match task.status {
                                    crate::domain::TaskStatus::Reviewed => MOCHA.green,
                                    crate::domain::TaskStatus::Ignored => MOCHA.overlay0,
                                    crate::domain::TaskStatus::Pending => MOCHA.yellow,
                                };
                                ui.label(
                                    egui::RichText::new(format!("Status: {:?}", task.status))
                                        .color(status_color),
                                );

                                // Risk indicator
                                let risk_color = match task.stats.risk {
                                    crate::domain::RiskLevel::Low => MOCHA.green,
                                    crate::domain::RiskLevel::Medium => MOCHA.yellow,
                                    crate::domain::RiskLevel::High => MOCHA.red,
                                };
                                ui.label(
                                    egui::RichText::new(format!("Risk: {:?}", task.stats.risk))
                                        .color(risk_color),
                                );

                                ui.label(format!("Files: {}", task.files.len()));
                                ui.label(format!(
                                    "Lines: +{} -{}",
                                    task.stats.additions, task.stats.deletions
                                ));
                            });

                            ui.separator();

                            // Task description
                            ui.label(egui::RichText::new("Description:").underline());
                            ui.label(&task.description);

                            // AI Insights if available
                            if let Some(insight) = &task.insight {
                                ui.separator();
                                ui.label(egui::RichText::new("AI Insight:").underline());
                                ui.label(insight);
                            }

                            ui.separator();

                            // Patches organized by file
                            ui.label(egui::RichText::new("Patches:").underline());
                            if task.patches.is_empty() {
                                ui.label("No patches to review");
                            } else {
                                // Group patches by file and display them separately in a stable order
                                use std::collections::HashMap;
                                let mut patches_by_file: HashMap<String, Vec<String>> =
                                    HashMap::new();

                                for patch in &task.patches {
                                    patches_by_file
                                        .entry(patch.file.clone())
                                        .or_insert_with(Vec::new)
                                        .push(patch.hunk.clone());
                                }

                                // Get sorted file paths for stable ordering
                                let mut sorted_files: Vec<String> =
                                    patches_by_file.keys().cloned().collect();
                                sorted_files.sort(); // Sort alphabetically for consistent ordering

                                // Display each file's patches separately
                                for file_path in &sorted_files {
                                    if let Some(hunks) = patches_by_file.get(file_path) {
                                        ui.push_id(file_path, |ui| {
                                            // Use push_id to avoid ID clashes
                                            ui.group(|ui| {
                                                ui.label(egui::RichText::new(file_path).strong());
                                                for (i, hunk) in hunks.iter().enumerate() {
                                                    if i > 0 {
                                                        ui.label("..."); // Indicate there are more hunks in the same file
                                                    }
                                                    // Use a fixed height for stability and avoid jumps
                                                    ui.push_id(
                                                        egui::Id::new((
                                                            "diff_editor",
                                                            file_path,
                                                            i,
                                                        )),
                                                        |ui| {
                                                            render_diff_editor(ui, hunk, "diff");
                                                        },
                                                    );
                                                }
                                            });
                                        });
                                    }
                                }
                            }

                            ui.separator();

                            // Notes section
                            ui.label(egui::RichText::new("Notes:").underline());
                            ui.horizontal(|ui| {
                                if ui.button("Save Note").clicked() {
                                    self.save_current_note();
                                }
                            });

                            if let Some(note_text) = &mut self.state.current_note {
                                ui.add(
                                    egui::TextEdit::multiline(note_text)
                                        .desired_rows(8)
                                        .desired_width(f32::INFINITY),
                                );
                            } else {
                                ui.label("No current note");
                            }
                        }
                    } else {
                        ui.label("Select a task from the navigation panel to view details");
                    }
                });
            });
        });
    }
}
