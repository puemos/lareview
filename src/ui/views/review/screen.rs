//! Review screen (task list + task detail)

use crate::application::review::ordering::{
    sub_flows_in_display_order, tasks_in_display_order, tasks_in_sub_flow_display_order,
};
use crate::ui::app::{Action, LaReviewApp, ReviewAction};
use crate::ui::components::action_button::action_button;
use crate::ui::components::pills::pill_action_button;
use crate::ui::components::status::error_banner;
use crate::ui::spacing;
use crate::ui::theme::current_theme;
use eframe::egui;
use egui_phosphor::regular as icons;

// Optimized header height for a "Toolbar" feel
const TOP_HEADER_HEIGHT: f32 = 52.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RightPaneState {
    TaskSelected,
    ReadyNoSelection,
    NoTasks,
    AllDone,
}

impl LaReviewApp {
    pub fn ui_review(&mut self, ui: &mut egui::Ui) {
        // --- 1. Data Preparation ---
        let tasks_by_sub_flow = self.state.tasks_by_sub_flow();
        let display_order_tasks = tasks_in_display_order(&tasks_by_sub_flow);
        let all_tasks = self.state.tasks();
        let total_tasks = display_order_tasks.len();

        let done_tasks = display_order_tasks
            .iter()
            .filter(|t| t.status.is_closed())
            .count();

        // Count open (Pending + InProgress)
        let open_tasks = display_order_tasks
            .iter()
            .filter(|t| {
                matches!(
                    t.status,
                    crate::domain::TaskStatus::Pending | crate::domain::TaskStatus::InProgress
                )
            })
            .count();

        // Find next actionable task
        let next_open_id = display_order_tasks
            .iter()
            .find(|t| t.status == crate::domain::TaskStatus::Pending)
            .or_else(|| {
                display_order_tasks
                    .iter()
                    .find(|t| t.status == crate::domain::TaskStatus::InProgress)
            })
            .map(|t| t.id.clone());

        let mut trigger_delete_review = false;

        // --- 2. Main Container Setup ---
        // We calculate the content area manually to handle the split view cleanly
        let side_margin = 0.0;
        let content_rect = ui
            .available_rect_before_wrap()
            .shrink2(egui::vec2(side_margin, 0.0));

        let mut content_ui = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));
        content_ui.set_clip_rect(content_rect); // Clip to prevent spillover
        let ui = &mut content_ui;

        // --- 3. Top Header (Toolbar) ---
        let header_response = egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(spacing::SPACING_MD as i8, 0))
            .show(ui, |ui| {
                ui.set_min_height(TOP_HEADER_HEIGHT);
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), TOP_HEADER_HEIGHT),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        // A. Left Side: Context Selectors
                        ui.horizontal(|ui| {
                            self.render_header_selectors(ui);
                        });

                        // B. Spacer
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // C. Right Side: Actions
                            ui.add_space(spacing::SPACING_XS);

                            // Delete Button
                            let review_selected = self.state.selected_review_id.is_some();
                            if review_selected {
                                if pill_action_button(
                                    ui,
                                    icons::TRASH_SIMPLE,
                                    "Delete",
                                    review_selected,
                                    current_theme().border,
                                )
                                .on_hover_text("Delete the selected review")
                                .clicked()
                                {
                                    trigger_delete_review = true;
                                }
                                ui.add_space(spacing::SPACING_XS);

                                // Export Button
                                if pill_action_button(
                                    ui,
                                    icons::EXPORT,
                                    "Export",
                                    review_selected,
                                    current_theme().border,
                                )
                                .on_hover_text("Export as markdown")
                                .clicked()
                                {
                                    self.dispatch(Action::Review(
                                        ReviewAction::RequestExportPreview,
                                    ));
                                }

                                if self.state.is_exporting {
                                    ui.add_space(spacing::SPACING_XS);
                                    ui.spinner();
                                }
                            }

                            // Progress Text (Right aligned next to actions)
                            if total_tasks > 0 {
                                ui.add_space(spacing::SPACING_MD);
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}/{} Tasks",
                                        done_tasks, total_tasks
                                    ))
                                    .size(12.0)
                                    .color(
                                        if done_tasks == total_tasks {
                                            current_theme().success
                                        } else {
                                            current_theme().text_muted
                                        },
                                    ),
                                );
                            }
                        });
                    },
                );
            });

        // Draw Full-Width Separator
        let bar_rect = egui::Rect::from_min_size(
            egui::pos2(
                content_rect.left(),
                header_response.response.rect.bottom() + 2.0,
            ),
            egui::vec2(content_rect.width(), 1.0),
        );
        ui.painter()
            .rect_filled(bar_rect, 0.0, current_theme().border);

        // Handle delayed actions
        if trigger_delete_review {
            self.dispatch(Action::Review(ReviewAction::DeleteReview));
            return;
        }

        // Error Banner
        if let Some(err) = &self.state.review_error {
            ui.add_space(6.0);
            error_banner(ui, err);
        }

        // --- 4. Content Area (Split View) ---

        // Handle "No Review" state early
        if display_order_tasks.is_empty() {
            self.render_empty_state(ui);
            return;
        }

        // Auto-selection logic
        self.handle_auto_selection(&display_order_tasks, next_open_id.clone());

        // Layout Calculations
        let available_rect = ui.available_rect_before_wrap();
        let available_height = available_rect.height();
        let available_width = available_rect.width();

        let tree_width_id = ui.id().with("tree_panel_width");
        let saved_tree_width = ui
            .memory(|mem| mem.data.get_temp::<f32>(tree_width_id))
            .unwrap_or(280.0);
        let max_tree_width = available_width * 0.45;
        let tree_width = crate::ui::layout::clamp_width(saved_tree_width, 220.0, max_tree_width);

        let resize_handle_width = 8.0; // wider logical hit-box, visual line will be thin

        // Rect Definitions
        // Safety: Ensure left pane + resize handle doesn't exceed available width
        let safe_tree_width = tree_width.min((available_width - resize_handle_width).max(0.0));

        let left_rect = egui::Rect::from_min_size(
            available_rect.min,
            egui::vec2(safe_tree_width, available_height),
        );
        let resize_rect = egui::Rect::from_min_size(
            egui::pos2(left_rect.max.x, left_rect.min.y),
            egui::vec2(resize_handle_width, available_height),
        );

        // Safety: Ensure center rect has non-negative width
        let center_min_x = resize_rect.max.x;
        let center_max_x = available_rect.max.x.max(center_min_x);

        let center_rect = egui::Rect::from_min_max(
            egui::pos2(center_min_x, available_rect.min.y),
            egui::pos2(center_max_x, available_rect.max.y),
        );

        // --- A. Left Panel (Navigation Tree) ---
        {
            let mut left_ui = ui.new_child(egui::UiBuilder::new().max_rect(left_rect));
            // Slight contrast for navigation panel
            egui::Frame::NONE
                .inner_margin(spacing::SPACING_MD)
                .show(&mut left_ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("nav_tree_scroll")
                        .show(ui, |ui| {
                            self.render_navigation_tree(ui, &tasks_by_sub_flow);
                        });
                });
        }

        // --- B. Resize Handle ---
        let resize_response = ui.interact(
            resize_rect,
            ui.id().with("resize_tree"),
            egui::Sense::drag(),
        );
        if resize_response.dragged()
            && let Some(pointer_pos) = ui.ctx().pointer_interact_pos()
        {
            let new_width = pointer_pos.x - available_rect.min.x;
            ui.memory_mut(|mem| mem.data.insert_temp(tree_width_id, new_width));
        }

        // Draw Resize Visuals
        let hover_active = resize_response.hovered() || resize_response.dragged();
        if hover_active {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        // Draw a subtle line in the center of the handle
        let line_x = resize_rect.center().x + 3.5;
        let line_color = if hover_active {
            current_theme().accent
        } else {
            current_theme().border
        };
        ui.painter().line_segment(
            [
                egui::pos2(line_x, resize_rect.min.y),
                egui::pos2(line_x, resize_rect.max.y),
            ],
            egui::Stroke::new(1.0, line_color),
        );

        // --- C. Center Panel (Task Detail or Status) ---
        {
            let mut center_ui = ui.new_child(egui::UiBuilder::new().max_rect(center_rect));
            egui::Frame::NONE
                .fill(current_theme().bg_primary)
                // REMOVED: .inner_margin(spacing::SPACING_XL) - We handle padding manually per view
                .show(&mut center_ui, |ui| {
                    self.render_center_pane(
                        ui,
                        &all_tasks,
                        display_order_tasks.len(),
                        open_tasks,
                        next_open_id,
                    );
                });
        }
    }

    /// Renders the dropdowns for Review and Run selection in the header
    fn render_header_selectors(&mut self, ui: &mut egui::Ui) {
        if self.state.reviews.is_empty() {
            return;
        }

        let current_id = self.state.selected_review_id.clone();
        let reviews = self.state.reviews.clone();

        // Find label
        let current_label = current_id
            .as_ref()
            .and_then(|id| reviews.iter().find(|r| &r.id == id))
            .map(|r| r.title.clone())
            .unwrap_or_else(|| "Select review…".to_string());

        // Review Selector
        ui.add(egui::Label::new(
            egui::RichText::new("Review:")
                .size(12.0)
                .color(current_theme().text_muted),
        ));
        egui::ComboBox::from_id_salt("review_select")
            .selected_text(egui::RichText::new(current_label).strong())
            .width(200.0)
            .show_ui(ui, |ui| {
                for review in &reviews {
                    let is_selected = current_id.as_deref() == Some(&review.id);
                    if ui.selectable_label(is_selected, &review.title).clicked() {
                        self.dispatch(Action::Review(ReviewAction::SelectReview {
                            review_id: review.id.clone(),
                        }));
                    }
                }
            });

        ui.add_space(spacing::SPACING_MD);

        // Run Selector (if Review selected)
        if let Some(selected_review_id) = self.state.selected_review_id.clone() {
            let runs: Vec<_> = self
                .state
                .runs
                .iter()
                .filter(|r| r.review_id == selected_review_id)
                .cloned()
                .collect();

            if !runs.is_empty() {
                let current_run_id = self.state.selected_run_id.clone();
                let run_label = current_run_id
                    .as_ref()
                    .and_then(|id| runs.iter().find(|r| &r.id == id))
                    .map(|r| format!("Run {}…", r.id.chars().take(6).collect::<String>()))
                    .unwrap_or_else(|| "Select run…".to_string());

                ui.add(egui::Label::new(
                    egui::RichText::new("Run:")
                        .size(12.0)
                        .color(current_theme().text_muted),
                ));
                egui::ComboBox::from_id_salt("run_select")
                    .selected_text(run_label)
                    .width(140.0)
                    .show_ui(ui, |ui| {
                        for run in runs {
                            let is_selected = current_run_id.as_deref() == Some(&run.id);
                            let label = format!(
                                "{}… ({})",
                                run.id.chars().take(8).collect::<String>(),
                                run.agent_id
                            );
                            if ui.selectable_label(is_selected, label).clicked() {
                                self.dispatch(Action::Review(ReviewAction::SelectRun {
                                    run_id: run.id.clone(),
                                }));
                            }
                        }
                    });
            }
        }
    }

    /// Renders the "No Tasks" empty state
    fn render_empty_state(&mut self, ui: &mut egui::Ui) {
        ui.allocate_ui_with_layout(
            ui.available_size(),
            egui::Layout::centered_and_justified(egui::Direction::TopDown),
            |ui| {
                ui.vertical_centered(|ui| {
                    // Hero Icon
                    ui.label(
                        egui::RichText::new(icons::BOUNDING_BOX)
                            .size(64.0)
                            .color(current_theme().border_secondary),
                    );
                    ui.add_space(spacing::SPACING_MD);
                    ui.heading("No review tasks yet");
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Generate tasks from your diff to start reviewing.")
                            .color(current_theme().text_muted),
                    );
                    ui.add_space(24.0);

                    if action_button(ui, "Generate tasks", true, current_theme().brand).clicked() {
                        self.switch_to_generate();
                    }
                });
            },
        );
    }

    /// Renders the logic for the Left Panel (Navigation)
    fn render_navigation_tree(
        &mut self,
        ui: &mut egui::Ui,
        tasks_by_sub_flow: &std::collections::HashMap<
            Option<String>,
            Vec<crate::domain::ReviewTask>,
        >,
    ) {
        let sub_flows = sub_flows_in_display_order(tasks_by_sub_flow);

        if sub_flows.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new("No tasks loaded")
                        .italics()
                        .color(current_theme().text_muted),
                );
            });
            return;
        }

        ui.spacing_mut().item_spacing = egui::vec2(0.0, spacing::SPACING_SM);
        ui.visuals_mut().indent_has_left_vline = false;

        for (sub_flow_name, tasks) in sub_flows {
            let title = sub_flow_name.as_deref().unwrap_or("UNCATEGORIZED");
            let title_upper = title.to_uppercase();
            let total = tasks.len();
            let finished = tasks.iter().filter(|t| t.status.is_closed()).count();
            let is_done = finished == total && total > 0;

            let header_id = ui.id().with(("sub_flow_collapse", title));

            ui.set_width(ui.available_width());

            egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                header_id,
                true,
            )
            .show_header(ui, |ui| {
                ui.horizontal(|ui| {
                    let mut heading = egui::RichText::new(&title_upper)
                        .family(egui::FontFamily::Proportional)
                        .strong()
                        .size(11.0)
                        .extra_letter_spacing(0.5);

                    if is_done {
                        heading = heading.color(current_theme().text_muted);
                    } else {
                        heading = heading.color(current_theme().text_primary);
                    }

                    ui.label(heading);

                    ui.add_space(spacing::SPACING_XS);

                    let color = if is_done {
                        current_theme().success
                    } else {
                        current_theme().text_muted
                    };

                    let count_text = egui::RichText::new(format!("{}/{}", finished, total))
                        .size(11.0)
                        .color(color);

                    ui.label(count_text);
                });
            })
            .body(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, spacing::SPACING_XS);
                for task in tasks_in_sub_flow_display_order(tasks) {
                    self.render_nav_item(ui, task);
                }
            });
        }
    }

    /// Renders the logic for the Center Panel
    fn render_center_pane(
        &mut self,
        ui: &mut egui::Ui,
        all_tasks: &[crate::domain::ReviewTask],
        total_count: usize,
        open_count: usize,
        next_open_id: Option<String>,
    ) {
        let state = if self.state.selected_task_id.is_some() {
            // Validate selection still exists
            if all_tasks
                .iter()
                .any(|t| Some(&t.id) == self.state.selected_task_id.as_ref())
            {
                RightPaneState::TaskSelected
            } else {
                RightPaneState::ReadyNoSelection // Fallback
            }
        } else if total_count == 0 {
            RightPaneState::NoTasks
        } else if open_count == 0 {
            RightPaneState::AllDone
        } else {
            RightPaneState::ReadyNoSelection
        };

        match state {
            RightPaneState::TaskSelected => {
                if let Some(task_id) = &self.state.selected_task_id
                    && let Some(task) = all_tasks.iter().find(|t| &t.id == task_id)
                {
                    self.render_task_detail(ui, task);
                }
            }
            RightPaneState::ReadyNoSelection => {
                self.render_ready_state(ui, next_open_id);
            }
            RightPaneState::AllDone => {
                egui::Frame::NONE
                    .inner_margin(spacing::SPACING_XL)
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(ui.available_height() * 0.3);
                            ui.label(
                                egui::RichText::new(icons::CHECK_CIRCLE)
                                    .size(64.0)
                                    .color(current_theme().success),
                            );
                            ui.add_space(16.0);
                            ui.heading("All tasks completed!");
                            ui.label(
                                egui::RichText::new("Great job.").color(current_theme().text_muted),
                            );
                        });
                    });
            }
            _ => {}
        }
    }

    /// Screen shown when tasks exist but none are selected
    fn render_ready_state(&mut self, ui: &mut egui::Ui, next_open_id: Option<String>) {
        // Keyboard shortcuts logic
        let mut trigger_primary = false;
        if ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift) {
            trigger_primary = true;
        }

        egui::Frame::NONE
            .inner_margin(spacing::SPACING_XL)
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.25);

                    // Hero Icon
                    ui.label(
                        egui::RichText::new(icons::LIST_CHECKS)
                            .size(64.0)
                            .color(current_theme().brand.gamma_multiply(0.8)),
                    );
                    ui.add_space(24.0);

                    ui.heading("Ready to Review");
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Select a task from the sidebar or start the queue.")
                            .color(current_theme().text_secondary),
                    );

                    ui.add_space(32.0);

                    // Primary Action
                    let btn_enabled = next_open_id.is_some();
                    let resp = pill_action_button(
                        ui,
                        icons::ARROW_RIGHT,
                        "Start Reviewing",
                        btn_enabled,
                        current_theme().brand,
                    );

                    // Hint for keyboard shortcut
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Press [Enter] to start")
                            .size(10.0)
                            .color(current_theme().text_disabled),
                    );

                    if (resp.clicked() || trigger_primary)
                        && btn_enabled
                        && let Some(id) = next_open_id
                        && let Some(task) = self.state.tasks().iter().find(|t| t.id == id)
                    {
                        // Helper to find task and select it
                        self.select_task(task);
                    }
                });
            });
    }

    fn handle_auto_selection(
        &mut self,
        display_tasks: &[&crate::domain::ReviewTask],
        _next_open_id: Option<String>,
    ) {
        // Only enforce logic if selection is invalid or missing when it shouldn't be
        let selection_valid = self
            .state
            .selected_task_id
            .as_ref()
            .is_some_and(|id| display_tasks.iter().any(|t| &t.id == id));

        if !selection_valid && !self.state.is_exporting {
            // Logic: If user was doing something, clear it. If just starting, maybe wait?
            // Current logic: clear if invalid.
            if self.state.selected_task_id.is_some() {
                self.dispatch(Action::Review(ReviewAction::ClearSelection));
            }
        }
    }
}
