use crate::ui::app::LaReviewApp;
use crate::ui::components::header::header;
use eframe::egui;

// diff editor helper
use crate::ui::components::diff::render_diff_editor;
use crate::ui::components::status::error_banner;
use catppuccin_egui::MOCHA;

/// Combines multiple patch hunks into a single unified diff string
/// (Kept from New, as it's more robust with header creation)
fn combine_patches_to_unified_diff(patches: &[crate::domain::Patch]) -> String {
    let mut sorted_patches = patches.to_vec();
    sorted_patches.sort_by(|a, b| a.file.cmp(&b.file));

    sorted_patches
        .iter()
        .map(|p| {
            let hunk = p.hunk.trim();
            if hunk.starts_with("diff --git") {
                hunk.to_string()
            } else {
                format!(
                    "diff --git a/{file} b/{file}\n--- a/{file}\n+++ b/{file}\n{hunk}",
                    file = p.file,
                    hunk = hunk
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

impl LaReviewApp {
    pub fn ui_review(&mut self, ui: &mut egui::Ui) {
        // 1. Top Global Header
        header(ui, "Review", None);

        // Error Banner
        if let Some(err) = &self.state.review_error {
            ui.add_space(6.0);
            error_banner(ui, err);
        }

        ui.add_space(8.0);

        // 2. Prepare Data
        let tasks_by_sub_flow = self.state.tasks_by_sub_flow();
        let all_tasks = self.state.tasks();
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

        // 3. Layout: Split View (Navigation Left | Content Right)
        let available_height = ui.available_height();

        // Memory for resizable width
        let tree_width_id = ui.id().with("tree_panel_width");
        let available_width = ui.available_width();
        let tree_width = ui
            .memory(|mem| mem.data.get_temp::<f32>(tree_width_id))
            .unwrap_or(300.0) // Slightly wider default for the Intent panel
            .clamp(200.0, available_width * 0.4);

        let (left_rect, right_rect) = {
            let available = ui.available_rect_before_wrap();
            let left =
                egui::Rect::from_min_size(available.min, egui::vec2(tree_width, available_height));
            // Add a small gap for the resize handle
            let right = egui::Rect::from_min_size(
                egui::pos2(available.min.x + tree_width + 4.0, available.min.y),
                egui::vec2(available.width() - tree_width - 4.0, available_height),
            );
            (left, right)
        };

        // --- LEFT PANEL: Navigation & Intent (Contains the Tree Layout) ---
        let mut left_ui = ui.new_child(egui::UiBuilder::new().max_rect(left_rect));
        egui::Frame::NONE
            .fill(MOCHA.mantle) // Darker background for sidebar
            .inner_margin(egui::Margin::same(12))
            .show(&mut left_ui, |ui| {
                // In ui_review, inside the left_ui frame:

                // PR Title: Make it bold and slightly larger than regular text
                ui.label(
                    egui::RichText::new(&self.state.pr_title)
                        .size(15.0)
                        .strong()
                        .color(MOCHA.text),
                );

                ui.add_space(12.0);

                // New Progress Bar & Metadata Row
                ui.horizontal(|ui| {
                    // Left: Progress Label
                    ui.label(
                        egui::RichText::new(format!("{} / {} Tasks", reviewed_tasks, total_tasks))
                            .color(MOCHA.subtext1)
                            .size(12.0),
                    );

                    // Right: Overall Risk Badge
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let max_risk = all_tasks
                            .iter()
                            .map(|t| t.stats.risk)
                            .max()
                            .unwrap_or(crate::domain::RiskLevel::Low);

                        let (risk_text, risk_bg, risk_fg) = match max_risk {
                            crate::domain::RiskLevel::Low => {
                                ("LOW RISK", MOCHA.green.gamma_multiply(0.2), MOCHA.green)
                            }
                            crate::domain::RiskLevel::Medium => {
                                ("MED RISK", MOCHA.yellow.gamma_multiply(0.2), MOCHA.yellow)
                            }
                            crate::domain::RiskLevel::High => {
                                ("HIGH RISK", MOCHA.red.gamma_multiply(0.2), MOCHA.red)
                            }
                        };
                        // NOTE: The 'badge' helper is assumed to be defined at the end of the file.
                        badge(ui, risk_text, risk_bg, risk_fg);
                    });
                });

                ui.add_space(4.0);

                // Progress Bar
                let progress_bar = egui::ProgressBar::new(progress).fill(if progress == 1.0 {
                    MOCHA.green
                } else {
                    MOCHA.blue
                });

                ui.add(progress_bar);

                ui.add_space(8.0);

                // PR Author Metadata
                ui.label(
                    egui::RichText::new(format!("Author: {}", self.state.pr_author))
                        .size(11.0)
                        .color(MOCHA.overlay1),
                );

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(12.0);

                // B. Navigation Tree (Sub-flows - The desired "tree layout" logic)
                egui::ScrollArea::vertical()
                    .id_salt("nav_tree_scroll")
                    .show(ui, |ui| {
                        let mut sub_flows: Vec<_> = tasks_by_sub_flow.iter().collect();
                        // Sort stable
                        sub_flows.sort_by(|(name_a, _), (name_b, _)| {
                            name_a
                                .as_deref()
                                .unwrap_or("ZZZ")
                                .cmp(name_b.as_deref().unwrap_or("ZZZ"))
                        });

                        for (sub_flow_name, tasks) in sub_flows {
                            let title = sub_flow_name.as_deref().unwrap_or("Uncategorized");
                            let finished_count = tasks
                                .iter()
                                .filter(|t| t.status == crate::domain::TaskStatus::Reviewed)
                                .count();
                            let total_tasks = tasks.len(); // <--- Get total count
                            let is_done = finished_count == total_tasks && !tasks.is_empty();

                            // --- Format the header text to include the counts ---
                            let header_title =
                                format!("{} ({}/{})", title, finished_count, total_tasks);
                            // ---

                            let mut header_text =
                                egui::RichText::new(header_title).color(if is_done {
                                    // <--- Use the new formatted string
                                    MOCHA.subtext0
                                } else {
                                    MOCHA.text
                                });
                            if is_done {
                                header_text = header_text.strikethrough();
                            }

                            egui::CollapsingHeader::new(header_text)
                                .id_salt(ui.id().with(("sub_flow", title)))
                                .default_open(true)
                                .show(ui, |ui| {
                                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 2.0);
                                    for task in tasks {
                                        self.render_nav_item(ui, task);
                                    }
                                });
                            ui.add_space(8.0);
                        }
                    });
            });

        // 4. Resize Handle
        let resize_rect = egui::Rect::from_min_size(
            egui::pos2(left_rect.max.x, left_rect.min.y),
            egui::vec2(4.0, left_rect.height()),
        );
        let resize_response = ui.interact(resize_rect, ui.id().with("resize"), egui::Sense::drag());

        if resize_response.dragged()
            && let Some(pointer_pos) = ui.ctx().pointer_interact_pos()
        {
            let new_width = (pointer_pos.x - left_rect.min.x).clamp(200.0, available_width * 0.6);
            ui.memory_mut(|mem| mem.data.insert_temp(tree_width_id, new_width));
        }

        // Draw resize line
        let line_color = if resize_response.hovered() || resize_response.dragged() {
            MOCHA.blue
        } else {
            MOCHA.surface0
        };
        ui.painter().rect_filled(resize_rect, 2.0, line_color);
        if resize_response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }

        // --- RIGHT PANEL: Task Details ---
        let mut right_ui = ui.new_child(egui::UiBuilder::new().max_rect(right_rect));
        egui::Frame::NONE
            .fill(ui.style().visuals.window_fill)
            .inner_margin(egui::Margin::symmetric(24, 16))
            .show(&mut right_ui, |ui| {
                if let Some(task_id) = &self.state.selected_task_id {
                    if let Some(task) = all_tasks.iter().find(|t| &t.id == task_id) {
                        self.render_task_detail(ui, task);
                    }
                } else {
                    // Empty State
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label(
                            egui::RichText::new(egui_phosphor::regular::LIST_CHECKS)
                                .size(64.0)
                                .color(MOCHA.surface2),
                        );
                        ui.add_space(16.0);
                        ui.heading("Select a Task to Review");
                        ui.label(
                            egui::RichText::new(
                                "Follow the sub-flows on the left to verify the intent.",
                            )
                            .color(MOCHA.subtext0),
                        );
                    });
                }
            });
    }

    /// Renders a single task item in the sidebar
    /// Renders a single task item in the sidebar
    fn render_nav_item(&mut self, ui: &mut egui::Ui, task: &crate::domain::ReviewTask) {
        let is_selected = self.state.selected_task_id.as_ref() == Some(&task.id);

        let (bg_color, text_color) = if is_selected {
            (MOCHA.surface1, MOCHA.text)
        } else {
            (egui::Color32::TRANSPARENT, MOCHA.subtext0)
        };

        let response = egui::Frame::NONE
            .fill(bg_color)
            .corner_radius(4.0)
            .inner_margin(4.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    // Risk Indicator Icon and Color
                    let (icon, color) = match task.stats.risk {
                        crate::domain::RiskLevel::Low => {
                            (egui_phosphor::regular::CIRCLE, MOCHA.green)
                        }
                        crate::domain::RiskLevel::Medium => {
                            (egui_phosphor::regular::CIRCLE_HALF, MOCHA.yellow)
                        }
                        crate::domain::RiskLevel::High => {
                            (egui_phosphor::regular::CIRCLE_DASHED, MOCHA.red)
                        }
                    };
                    ui.label(egui::RichText::new(icon).size(16.0).color(color));

                    ui.add_space(6.0);

                    // Title
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&task.title)
                                .color(text_color)
                                .size(13.0),
                        )
                        .truncate(),
                    );
                })
                .response
            })
            .response;

        // --- Cursor and Click Logic ---
        let interact_response = response.interact(egui::Sense::click());

        if interact_response.hovered() {
            // Set cursor to pointer (hand) when the item is hovered
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        if interact_response.clicked() {
            self.state.selected_task_id = Some(task.id.clone());
            // Load note logic
            if let Ok(Some(note)) = self.note_repo.find_by_task(&task.id) {
                self.state.current_note = Some(note.body);
            } else {
                self.state.current_note = Some(String::new());
            }
        }
    }

    /// Renders the detailed view of the selected task
    fn render_task_detail(&mut self, ui: &mut egui::Ui, task: &crate::domain::ReviewTask) {
        egui::ScrollArea::vertical()
            .id_salt("detail_scroll")
            .show(ui, |ui| {
                // 1. Task Header
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new(&task.title)
                            .size(24.0)
                            .color(MOCHA.text),
                    );
                });

                ui.add_space(8.0);

                // 2. Metadata Badges
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

                    // Status Badge
                    let (status_text, status_bg, status_fg) = match task.status {
                        crate::domain::TaskStatus::Reviewed => {
                            ("REVIEWED", MOCHA.green, MOCHA.base)
                        }
                        crate::domain::TaskStatus::Ignored => {
                            ("IGNORED", MOCHA.surface2, MOCHA.text)
                        }
                        crate::domain::TaskStatus::Pending => ("PENDING", MOCHA.yellow, MOCHA.base),
                    };
                    badge(ui, status_text, status_bg, status_fg);

                    // Risk Badge
                    let (risk_text, risk_bg, risk_fg) = match task.stats.risk {
                        crate::domain::RiskLevel::Low => {
                            ("LOW RISK", MOCHA.green.gamma_multiply(0.2), MOCHA.green)
                        }
                        crate::domain::RiskLevel::Medium => {
                            ("MED RISK", MOCHA.yellow.gamma_multiply(0.2), MOCHA.yellow)
                        }
                        crate::domain::RiskLevel::High => {
                            ("HIGH RISK", MOCHA.red.gamma_multiply(0.2), MOCHA.red)
                        }
                    };
                    badge(ui, risk_text, risk_bg, risk_fg);

                    // File/Line Stats
                    let stats_text = format!(
                        "{} files |+{} -{} lines",
                        task.files.len(),
                        task.stats.additions,
                        task.stats.deletions
                    );
                    badge(ui, &stats_text, MOCHA.surface0, MOCHA.subtext0);
                });

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

                // 4. Diff Viewer
                ui.label(egui::RichText::new("Changes").strong().size(16.0));

                let cache_id = ui.id().with("unified_diff_cache").with(&task.id);
                let unified_diff = ui.ctx().memory_mut(|mem| {
                    mem.data
                        .get_temp_mut_or_insert_with(cache_id, || {
                            combine_patches_to_unified_diff(&task.patches)
                        })
                        .clone()
                });

                if task.patches.is_empty() {
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
                                let action = render_diff_editor(ui, &unified_diff, "diff");
                                if matches!(
                                    action,
                                    crate::ui::components::DiffAction::OpenFullWindow
                                ) {
                                    self.state.full_diff = Some(crate::ui::app::FullDiffView {
                                        title: format!("Task diff - {}", task.title),
                                        text: unified_diff.clone(),
                                    });
                                }
                            });
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

// Helper for drawing badges
fn badge(ui: &mut egui::Ui, text: &str, bg: egui::Color32, fg: egui::Color32) {
    egui::Frame::NONE
        .fill(bg)
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).size(10.0).strong().color(fg));
        });
}
