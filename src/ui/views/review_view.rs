//! Review view (egui version) - Left-Right Tree Layout
use crate::ui::app::LaReviewApp;
use crate::ui::components::header::header;
use eframe::egui;

// diff editor helper
use crate::ui::components::diff::render_diff_editor;
use crate::ui::components::status::error_banner;
use catppuccin_egui::MOCHA;

/// Combines multiple patch hunks into a single unified diff string
/// Each patch.hunk is already a complete git diff for that file
fn combine_patches_to_unified_diff(patches: &[crate::domain::Patch]) -> String {
    // Sort patches by file path for stable ordering
    let mut sorted_patches = patches.to_vec();
    sorted_patches.sort_by(|a, b| a.file.cmp(&b.file));

    // Concatenate all the diffs with double newlines between files (standard format)
    sorted_patches
        .iter()
        .map(|p| p.hunk.trim())
        .collect::<Vec<_>>()
        .join("\n")
}

impl LaReviewApp {
    pub fn ui_review(&mut self, ui: &mut egui::Ui) {
        header(ui, "Review", None);
        ui.add_space(6.0);

        // show error if any
        if let Some(err) = &self.state.review_error {
            error_banner(ui, err);
            ui.add_space(6.0);
        }

        // PR Header
        ui.group(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(12.0, 4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(format!("PR: {}", self.state.pr_title))
                        .strong()
                        .size(14.0),
                );
                ui.add_space(12.0);
                ui.label(format!("Repo: {}", self.state.pr_repo));
                ui.add_space(8.0);
                ui.label(format!("Author: {}", self.state.pr_author));
                ui.add_space(8.0);
                ui.label(format!("Branch: {}", self.state.pr_branch));
            });
        });

        ui.add_space(8.0);

        // Group tasks by sub flows
        let tasks_by_sub_flow = self.state.tasks_by_sub_flow();
        let all_tasks = self.state.tasks();

        // Overall progress
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

        // Intent panel
        egui::Frame::group(ui.style())
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 4.0);

                ui.heading(egui::RichText::new("INTENT").size(16.0).color(MOCHA.mauve));
                ui.add_space(4.0);

                ui.label(
                    egui::RichText::new(&self.state.pr_title)
                        .strong()
                        .size(14.0),
                );

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Status: {}/{} tasks reviewed",
                        reviewed_tasks, total_tasks
                    ));
                    ui.add_space(12.0);
                    ui.label(format!("Progress: {:.0}%", progress * 100.0));
                });

                let overall_risk = if all_tasks.is_empty() {
                    crate::domain::RiskLevel::Low
                } else {
                    all_tasks
                        .iter()
                        .map(|t| t.stats.risk)
                        .max()
                        .unwrap_or(crate::domain::RiskLevel::Low)
                };
                ui.add_space(4.0);
                ui.label(format!("Risk: {:?}", overall_risk));
            });

        ui.add_space(10.0);

        // Stable ordering of sub flows
        let mut sub_flows: Vec<(&Option<String>, &Vec<crate::domain::ReviewTask>)> =
            tasks_by_sub_flow.iter().collect();
        sub_flows.sort_by(|(name_a, _), (name_b, _)| {
            name_a
                .as_deref()
                .unwrap_or("ZZZ")
                .cmp(name_b.as_deref().unwrap_or("ZZZ"))
        });

        // Tree width in memory
        let tree_width_id = ui.id().with("tree_panel_width");
        let available_width = ui.available_width();

        let tree_width = ui
            .memory(|mem| mem.data.get_temp::<f32>(tree_width_id))
            .unwrap_or(260.0)
            .clamp(170.0, available_width * 0.5);

        let (left_rect, right_rect) = {
            let available = ui.available_rect_before_wrap();

            let left = egui::Rect::from_min_size(
                available.min,
                egui::vec2(tree_width, available.height()),
            );

            let right = egui::Rect::from_min_size(
                egui::pos2(available.min.x + tree_width, available.min.y),
                egui::vec2(available.width() - tree_width, available.height()),
            );

            (left, right)
        };

        // Left panel - navigation
        let mut left_ui = ui.new_child(egui::UiBuilder::new().max_rect(left_rect));
        {
            egui::Frame::default()
                .fill(left_ui.style().visuals.window_fill)
                .inner_margin(egui::Margin::same(8))
                .show(&mut left_ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 6.0);

                    ui.heading("Tasks");
                    ui.add_space(4.0);

                    egui::ScrollArea::vertical()
                        .id_salt(ui.id().with("tasks_scroll"))
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            for (idx, (sub_flow_name, tasks)) in sub_flows.iter().enumerate() {
                                if idx > 0 {
                                    ui.add_space(6.0);
                                }

                                let sub_flow_title =
                                    sub_flow_name.as_deref().unwrap_or("Uncategorized");

                                let sub_total = tasks.len();
                                let sub_reviewed = tasks
                                    .iter()
                                    .filter(|t| t.status == crate::domain::TaskStatus::Reviewed)
                                    .count();

                                egui::CollapsingHeader::new(format!(
                                    "{} ({}/{})",
                                    sub_flow_title, sub_reviewed, sub_total
                                ))
                                .id_salt(ui.id().with(("sub_flow", sub_flow_title)))
                                .show(ui, |ui| {
                                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);

                                    for task in tasks.iter() {
                                        let is_selected =
                                            self.state.selected_task_id.as_ref() == Some(&task.id);

                                        ui.horizontal(|ui| {
                                            let risk_color = match task.stats.risk {
                                                crate::domain::RiskLevel::Low => MOCHA.green,
                                                crate::domain::RiskLevel::Medium => MOCHA.yellow,
                                                crate::domain::RiskLevel::High => MOCHA.red,
                                            };
                                            ui.label(
                                                egui::RichText::new(match task.stats.risk {
                                                    crate::domain::RiskLevel::Low => {
                                                        egui_phosphor::regular::CIRCLE
                                                    }
                                                    crate::domain::RiskLevel::Medium => {
                                                        egui_phosphor::regular::CIRCLE_HALF
                                                    }
                                                    crate::domain::RiskLevel::High => {
                                                        egui_phosphor::regular::CIRCLE_DASHED
                                                    }
                                                })
                                                .color(risk_color),
                                            );

                                            let selectable =
                                                ui.selectable_label(is_selected, &task.title);

                                            if selectable.clicked() {
                                                self.state.selected_task_id = Some(task.id.clone());

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

                                        ui.add_space(2.0);
                                    }
                                });
                            }
                        });
                });
        }

        // Resize handle
        let resize_id = ui.id().with("resize_handle");
        let resize_rect = egui::Rect::from_min_size(
            egui::pos2(left_rect.max.x - 2.0, left_rect.min.y),
            egui::vec2(4.0, left_rect.height()),
        );

        let resize_response = ui.interact(resize_rect, resize_id, egui::Sense::drag());

        if resize_response.dragged()
            && let Some(pointer_pos) = ui.ctx().pointer_interact_pos()
        {
            let new_width = (pointer_pos.x - left_rect.min.x).clamp(170.0, available_width * 0.7);
            ui.memory_mut(|mem| {
                mem.data.insert_temp(tree_width_id, new_width);
            });
        }

        let handle_color = if resize_response.hovered() || resize_response.dragged() {
            ui.style().visuals.widgets.active.bg_fill
        } else {
            ui.style().visuals.widgets.inactive.bg_fill
        };
        ui.painter().rect_filled(resize_rect, 0.0, handle_color);

        if resize_response.hovered() || resize_response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }

        // Right panel - content
        let mut right_ui = ui.new_child(egui::UiBuilder::new().max_rect(right_rect));
        {
            egui::Frame::default()
                .fill(right_ui.style().visuals.window_fill)
                .inner_margin(egui::Margin::same(10))
                .show(&mut right_ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt(ui.id().with("task_detail_scroll"))
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(8.0, 10.0);

                            if let Some(task_id) = &self.state.selected_task_id {
                                if let Some(task) = all_tasks.iter().find(|t| &t.id == task_id) {
                                    ui.heading(
                                        egui::RichText::new(&task.title)
                                            .size(16.0)
                                            .color(MOCHA.text),
                                    );

                                    ui.add_space(4.0);

                                    ui.horizontal(|ui| {
                                        let status_color = match task.status {
                                            crate::domain::TaskStatus::Reviewed => MOCHA.green,
                                            crate::domain::TaskStatus::Ignored => {
                                                MOCHA.overlay0
                                            }
                                            crate::domain::TaskStatus::Pending => MOCHA.yellow,
                                        };
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "Status: {:?}",
                                                task.status
                                            ))
                                            .color(status_color),
                                        );

                                        let risk_color = match task.stats.risk {
                                            crate::domain::RiskLevel::Low => MOCHA.green,
                                            crate::domain::RiskLevel::Medium => MOCHA.yellow,
                                            crate::domain::RiskLevel::High => MOCHA.red,
                                        };
                                        ui.add_space(12.0);
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "Risk: {:?}",
                                                task.stats.risk
                                            ))
                                            .color(risk_color),
                                        );

                                        ui.add_space(12.0);
                                        ui.label(format!("Files: {}", task.files.len()));
                                        ui.add_space(8.0);
                                        ui.label(format!(
                                            "Lines: +{} -{}",
                                            task.stats.additions, task.stats.deletions
                                        ));
                                    });

                                    ui.separator();

                                    ui.label(
                                        egui::RichText::new("Description")
                                            .underline()
                                            .strong(),
                                    );
                                    ui.label(&task.description);

                                    if let Some(insight) = &task.insight {
                                        ui.add_space(6.0);
                                        ui.label(
                                            egui::RichText::new("AI insight")
                                                .underline()
                                                .strong(),
                                        );
                                        ui.label(insight);
                                    }

                                    ui.separator();

                                    let cache_id =
                                        ui.id().with("unified_diff_cache").with(&task.id);

                                    let unified_diff = ui.ctx().memory_mut(|mem| {
                                        mem.data
                                            .get_temp_mut_or_insert_with(cache_id, || {
                                                combine_patches_to_unified_diff(&task.patches)
                                            })
                                            .clone()
                                    });

                                    ui.label(
                                        egui::RichText::new("Patches")
                                            .underline()
                                            .strong(),
                                    );
                                    if task.patches.is_empty() {
                                        ui.label("No patches to review");
                                    } else {
                                        ui.push_id(("unified_diff", &task.id), |ui| {
                                            let action = render_diff_editor(ui, &unified_diff, "diff");

                                            if matches!(action, crate::ui::components::DiffAction::OpenFullWindow) {
                                                self.state.full_diff = Some(crate::ui::app::FullDiffView {
                                                    title: format!("Task diff - {}", task.title),
                                                    source: crate::ui::app::FullDiffSource::ReviewTask {
                                                        task_id: task.id.clone(),
                                                    },
                                                    text: unified_diff.clone(),
                                                });
                                            }
                                        });
                                    }

                                    ui.separator();

                                    ui.label(
                                        egui::RichText::new("Notes")
                                            .underline()
                                            .strong(),
                                    );
                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        if ui.button("Save note").clicked() {
                                            self.save_current_note();
                                        }
                                    });

                                    if let Some(note_text) = &mut self.state.current_note {
                                        ui.add(
                                            egui::TextEdit::multiline(note_text)
                                                .id_salt(ui.id().with((
                                                    "task_note",
                                                    &task.id,
                                                )))
                                                .desired_rows(8)
                                                .desired_width(f32::INFINITY),
                                        );
                                    } else {
                                        ui.label("No current note");
                                    }
                                }
                            } else {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(40.0);
                                    ui.label(
                                        egui::RichText::new(
                                            "Select a task from the navigation panel to view details",
                                        )
                                        .italics(),
                                    );
                                });
                            }
                        });
                });
        }
    }
}
