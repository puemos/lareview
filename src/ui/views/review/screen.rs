//! Review screen (task list + task detail)

use crate::application::review::ordering::{
    sub_flows_in_display_order, tasks_in_display_order, tasks_in_sub_flow_display_order,
};
use crate::ui::app::{Action, LaReviewApp, ReviewAction};
use crate::ui::components::header::action_button;
use crate::ui::components::status::error_banner;
use crate::ui::components::{badge::badge, pills::pill_action_button};
use catppuccin_egui::MOCHA;
use eframe::egui;
use egui_phosphor::regular as icons;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RightPaneState {
    TaskSelected,
    ReadyNoSelection,
    NoTasks,
    AllDone,
}

impl LaReviewApp {
    pub fn ui_review(&mut self, ui: &mut egui::Ui) {
        // Prepare data upfront so the header can show progress/actions.
        let tasks_by_sub_flow = self.state.tasks_by_sub_flow();
        let display_order_tasks = tasks_in_display_order(&tasks_by_sub_flow);
        let all_tasks = self.state.tasks();
        let total_tasks = display_order_tasks.len();
        let done_tasks = display_order_tasks
            .iter()
            .filter(|t| {
                matches!(
                    t.status,
                    crate::domain::TaskStatus::Done | crate::domain::TaskStatus::Ignored
                )
            })
            .count();
        let open_tasks = display_order_tasks
            .iter()
            .filter(|t| {
                matches!(
                    t.status,
                    crate::domain::TaskStatus::Pending | crate::domain::TaskStatus::InProgress
                )
            })
            .count();

        let progress = if total_tasks > 0 {
            (done_tasks as f32) / (total_tasks as f32)
        } else {
            0.0
        };

        let has_done_tasks = display_order_tasks
            .iter()
            .any(|t| t.status == crate::domain::TaskStatus::Done);

        let next_open_id = display_order_tasks
            .iter()
            .find(|t| t.status == crate::domain::TaskStatus::Pending)
            .or_else(|| {
                display_order_tasks
                    .iter()
                    .find(|t| t.status == crate::domain::TaskStatus::InProgress)
            })
            .map(|t| t.id.clone());

        let mut trigger_clean_done = false;
        let mut trigger_next_open = false;

        // Top header (minimal, Linear-ish).
        ui.horizontal(|ui| {
            ui.heading(egui::RichText::new("Review").size(18.0).color(MOCHA.text));

            if total_tasks > 0 {
                ui.add_space(10.0);
                badge(
                    ui,
                    &format!("{done_tasks}/{total_tasks} done"),
                    MOCHA.surface0,
                    MOCHA.subtext0,
                );
            }

            if total_tasks > 0 {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let resp = pill_action_button(
                        ui,
                        icons::TRASH_SIMPLE,
                        "Clean done",
                        has_done_tasks,
                        MOCHA.red,
                    )
                    .on_hover_text("Remove DONE tasks (and their notes) for this PR");
                    if resp.clicked() {
                        trigger_clean_done = true;
                    }

                    ui.add_space(8.0);

                    let next_enabled = next_open_id.is_some();
                    let resp = pill_action_button(
                        ui,
                        icons::ARROW_RIGHT,
                        "Next open",
                        next_enabled,
                        MOCHA.mauve,
                    )
                    .on_hover_text("Jump to the next open task");
                    if resp.clicked() {
                        trigger_next_open = true;
                    }
                });
            }
        });
        ui.add_space(6.0);

        if trigger_clean_done {
            self.clean_done_tasks();
            return;
        }

        if trigger_next_open && let Some(id) = next_open_id.as_deref() {
            self.select_task_by_id(&all_tasks, id);
            return;
        }

        // Error Banner
        if let Some(err) = &self.state.review_error {
            ui.add_space(6.0);
            error_banner(ui, err);
        }

        ui.add_space(8.0);

        // 3. Default selection rule (meaningful auto-select only)
        let selected_task_is_valid = self
            .state
            .selected_task_id
            .as_ref()
            .is_some_and(|id| display_order_tasks.iter().any(|t| &t.id == id));

        if display_order_tasks.is_empty() {
            if self.state.selected_task_id.is_some()
                || self.state.current_note.is_some()
                || self.state.current_line_note.is_some()
            {
                self.dispatch(Action::Review(ReviewAction::ClearSelection));
            }
        } else if !selected_task_is_valid {
            if let Some(next_open) = display_order_tasks
                .iter()
                .find(|t| t.status == crate::domain::TaskStatus::Pending)
                .or_else(|| {
                    display_order_tasks
                        .iter()
                        .find(|t| t.status == crate::domain::TaskStatus::InProgress)
                })
            {
                self.select_task(next_open);
            } else {
                // No pending tasks: show "All done" by default (do not auto-select done).
                if self.state.selected_task_id.is_some()
                    || self.state.current_note.is_some()
                    || self.state.current_line_note.is_some()
                {
                    self.dispatch(Action::Review(ReviewAction::ClearSelection));
                }
            }
        }

        if display_order_tasks.is_empty() {
            let available = ui.available_size();
            ui.allocate_ui_with_layout(
                available,
                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("No review tasks yet");
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(
                                "Generate tasks from your diff to start reviewing.",
                            )
                            .color(MOCHA.subtext0),
                        );
                        ui.add_space(16.0);
                        let cta = egui::Button::new(
                            egui::RichText::new("Generate tasks")
                                .size(15.0)
                                .color(MOCHA.mauve),
                        )
                        .fill(egui::Color32::TRANSPARENT)
                        .stroke(egui::Stroke::NONE);
                        if ui.add(cta).clicked() {
                            self.switch_to_generate();
                        }
                    });
                },
            );
            return;
        }

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
                if total_tasks == 0 {
                    ui.vertical_centered(|ui| {
                        ui.add_space(60.0);
                        ui.label(
                            egui::RichText::new(egui_phosphor::regular::BOUNDING_BOX)
                                .size(48.0)
                                .color(MOCHA.surface2),
                        );
                        ui.add_space(12.0);
                        ui.heading("No review tasks yet");
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(
                                "Once tasks are generated, they will show up in the left panel.",
                            )
                            .color(MOCHA.subtext0),
                        );
                        ui.add_space(14.0);
                        if action_button(ui, "Generate tasks", true, MOCHA.mauve).clicked() {
                            self.switch_to_generate();
                        }
                    });
                    return;
                }

                // PR Title: Make it bold and slightly larger than regular text
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&self.state.pr_title)
                            .size(15.0)
                            .strong()
                            .color(MOCHA.text),
                    )
                    .wrap(),
                );

                ui.add_space(12.0);

                // New Progress Bar & Metadata Row
                ui.horizontal(|ui| {
                    // Left: Progress Label
                    ui.label(
                        egui::RichText::new(format!("{} / {} Tasks", done_tasks, total_tasks))
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

                        let (risk_icon, risk_text, risk_bg, risk_fg) = match max_risk {
                            crate::domain::RiskLevel::Low => (
                                icons::CARET_CIRCLE_DOWN,
                                "Low risk",
                                MOCHA.blue.gamma_multiply(0.2),
                                MOCHA.blue,
                            ),
                            crate::domain::RiskLevel::Medium => (
                                icons::CARET_CIRCLE_UP,
                                "Med risk",
                                MOCHA.yellow.gamma_multiply(0.2),
                                MOCHA.yellow,
                            ),
                            crate::domain::RiskLevel::High => (
                                icons::CARET_CIRCLE_DOUBLE_UP,
                                "High risk",
                                MOCHA.red.gamma_multiply(0.2),
                                MOCHA.red,
                            ),
                        };
                        // NOTE: The 'badge' helper is assumed to be defined at the end of the file.
                        badge(ui, &format!("{risk_icon} {risk_text}"), risk_bg, risk_fg);
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

                // B. Navigation Tree (Sub-flows)
                egui::ScrollArea::vertical()
                    .id_salt("nav_tree_scroll")
                    .show(ui, |ui| {
                        for (sub_flow_name, tasks) in sub_flows_in_display_order(&tasks_by_sub_flow)
                        {
                            let title = sub_flow_name.as_deref().unwrap_or("Uncategorized");
                            let finished_count =
                                tasks.iter().filter(|t| t.status.is_closed()).count();
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
                                    for task in tasks_in_sub_flow_display_order(tasks) {
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
                let right_state = if display_order_tasks.is_empty() {
                    RightPaneState::NoTasks
                } else if self.state.selected_task_id.is_some() {
                    RightPaneState::TaskSelected
                } else if open_tasks == 0 {
                    RightPaneState::AllDone
                } else {
                    RightPaneState::ReadyNoSelection
                };

                // Empty states + detail
                match right_state {
                    RightPaneState::TaskSelected => {
                        if let Some(task_id) = &self.state.selected_task_id
                            && let Some(task) = all_tasks.iter().find(|t| &t.id == task_id) {
                            self.render_task_detail(ui, task);
                            return;
                        }
                        // Selection is missing from current list: fall back to Ready state.
                        self.dispatch(Action::Review(ReviewAction::ClearSelection));
                    }
                    RightPaneState::NoTasks => {}
                    RightPaneState::AllDone => {}
                    RightPaneState::ReadyNoSelection => {}
                }

                // Track state transitions for focus behavior.
                let pane_state_id = ui.id().with("right_pane_state");
                let prev_state = ui
                    .memory(|mem| mem.data.get_temp::<RightPaneState>(pane_state_id))
                    .unwrap_or(RightPaneState::TaskSelected);
                ui.memory_mut(|mem| mem.data.insert_temp(pane_state_id, right_state));

                match right_state {
                    RightPaneState::ReadyNoSelection => {
                        let should_focus_primary =
                            prev_state != RightPaneState::ReadyNoSelection
                                && ui.ctx().memory(|mem| mem.focused().is_none());

                        // Keyboard shortcuts (only in this state).
                        let mut trigger_primary = false;
                        let mut trigger_secondary = false;
                        ui.ctx().input(|i| {
                            if i.key_pressed(egui::Key::Enter) {
                                if i.modifiers.shift {
                                    trigger_secondary = true;
                                } else {
                                    trigger_primary = true;
                                }
                            }
                        });

                        ui.vertical_centered(|ui| {
                            ui.add_space(80.0);
                            ui.label(
                                egui::RichText::new(egui_phosphor::regular::LIST_CHECKS)
                                    .size(64.0)
                                    .color(MOCHA.surface2),
                            );
                            ui.add_space(16.0);
                            ui.heading("Ready to review");
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new("Pick a task on the left, or jump in now.")
                                    .color(MOCHA.subtext0),
                            );
                            ui.add_space(16.0);

                            let primary_enabled = next_open_id.is_some();
                            let primary_resp =
                                pill_action_button(ui, icons::ARROW_RIGHT, "Next open", primary_enabled, MOCHA.mauve);
                            if should_focus_primary {
                                ui.memory_mut(|mem| mem.request_focus(primary_resp.id));
                            }
                            if (primary_resp.clicked() || trigger_primary)
                                && let Some(id) = next_open_id.as_deref()
                            {
                                self.select_task_by_id(&all_tasks, id);
                            }

                            ui.add_space(12.0);
                            ui.label(
                                egui::RichText::new(
                                    "Tip: Start with HIGH RISK to catch big issues early.",
                                )
                                .color(MOCHA.subtext0)
                                .size(12.0),
                            );
                        });
                    }
                    RightPaneState::NoTasks => {
                        ui.vertical_centered(|ui| {
                            ui.add_space(80.0);
                            ui.label(
                                egui::RichText::new(egui_phosphor::regular::BOUNDING_BOX)
                                    .size(64.0)
                                    .color(MOCHA.surface2),
                            );
                            ui.add_space(16.0);
                            ui.heading("No review tasks yet");
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new(
                                    "Once tasks are generated, they will show up in the left panel.",
                                )
                                .color(MOCHA.subtext0),
                            );
                            ui.add_space(16.0);
                            if action_button(ui, "Generate tasks", true, MOCHA.mauve).clicked() {
                                self.switch_to_generate();
                            }
                        });
                    }
                    RightPaneState::AllDone => {
                        ui.vertical_centered(|ui| {
                            ui.add_space(80.0);
                            ui.label(
                                egui::RichText::new(egui_phosphor::regular::CHECK_CIRCLE)
                                    .size(64.0)
                                    .color(MOCHA.green.gamma_multiply(0.8)),
                            );
                            ui.add_space(16.0);
                            ui.heading("All done");
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new(format!("You closed {} tasks.", done_tasks))
                                    .color(MOCHA.subtext0),
                            );
                            ui.add_space(16.0);
                            if action_button(ui, "Back to generate", true, MOCHA.mauve).clicked() {
                                self.switch_to_generate();
                            }
                        });
                    }
                    RightPaneState::TaskSelected => {
                        // handled above
                    }
                }
            });
    }
}
