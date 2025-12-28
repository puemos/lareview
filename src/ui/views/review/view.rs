//! Review screen (task list + task detail)

use crate::application::review::ordering::tasks_in_display_order;
use crate::ui::app::{Action, LaReviewApp, ReviewAction};
use crate::ui::components::pills::pill_action_button;
use crate::ui::components::status::error_banner;
use crate::ui::spacing::TOP_HEADER_HEIGHT;
use crate::ui::theme::current_theme;
use crate::ui::{icons, spacing, typography};
use eframe::egui;
use egui::Margin;
use egui::epaint::MarginF32;

use crate::ui::views::review::nav::render_navigation_tree;
use crate::ui::views::review::toolbar::render_header_selectors;

impl LaReviewApp {
    pub fn ui_review(&mut self, ui: &mut egui::Ui) {
        if ui.available_width() < 100.0 {
            return;
        }

        let theme = current_theme();

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
                    crate::domain::ReviewStatus::Todo | crate::domain::ReviewStatus::InProgress
                )
            })
            .count();

        // Find next actionable task
        let next_open_id = display_order_tasks
            .iter()
            .find(|t| t.status == crate::domain::ReviewStatus::Todo)
            .or_else(|| {
                display_order_tasks
                    .iter()
                    .find(|t| t.status == crate::domain::ReviewStatus::InProgress)
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
        let _header_response = egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(spacing::SPACING_XL as i8, 0))
            .show(ui, |ui| {
                ui.set_min_height(TOP_HEADER_HEIGHT);
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), TOP_HEADER_HEIGHT),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        // A. Left Side: Context Selectors
                        ui.horizontal(|ui| {
                            if let Some(action) = render_header_selectors(
                                ui,
                                &self.state.domain.reviews,
                                self.state.ui.selected_review_id.as_ref(),
                                &theme,
                            ) {
                                self.dispatch(Action::Review(action));
                            }

                            if self.state.session.is_generating
                                && self.state.ui.selected_review_id
                                    == self.state.session.generating_review_id
                            {
                                ui.add_space(spacing::SPACING_XS);
                                crate::ui::animations::cyber::cyber_spinner(
                                    ui,
                                    theme.brand,
                                    Some(crate::ui::animations::cyber::CyberSpinnerSize::Sm),
                                );
                            }
                        });

                        // B. Spacer
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // C. Right Side: Actions
                            ui.add_space(spacing::SPACING_XS);

                            // Delete Button
                            let review_selected = self.state.ui.selected_review_id.is_some();
                            if review_selected {
                                if pill_action_button(
                                    ui,
                                    icons::ACTION_DELETE,
                                    "Delete",
                                    review_selected,
                                    theme.border,
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
                                    icons::ACTION_EXPORT,
                                    "Export Review...",
                                    review_selected,
                                    theme.border,
                                )
                                .on_hover_text("Export Review...")
                                .clicked()
                                {
                                    self.dispatch(Action::Review(
                                        ReviewAction::RequestExportPreview,
                                    ));
                                }
                            }
                            // Progress Text (Right aligned next to actions)
                            if total_tasks > 0 {
                                ui.add_space(spacing::SPACING_MD);
                                ui.label(
                                    typography::body(format!(
                                        "{}/{} Tasks",
                                        done_tasks, total_tasks
                                    ))
                                    .size(12.0)
                                    .color(
                                        if done_tasks == total_tasks {
                                            theme.success
                                        } else {
                                            theme.text_muted
                                        },
                                    ),
                                );
                            }
                        });
                    },
                );
            });

        // Draw Full-Width Separator only if we have tasks or are generating
        let is_generating_this = self.state.session.is_generating
            && self.state.ui.selected_review_id == self.state.session.generating_review_id;

        if total_tasks > 0 || is_generating_this {
            ui.separator();
        }

        // Handle delayed actions
        if trigger_delete_review {
            if let Some(id) = &self.state.ui.selected_review_id {
                self.dispatch(Action::Review(ReviewAction::DeleteReview(id.clone())));
            }
            return;
        }

        // Error Banner
        if let Some(err) = &self.state.ui.review_error {
            ui.add_space(6.0);
            error_banner(ui, err);
        }

        // --- 4. Content Area (Split View) ---

        // Handle "No Review" state early
        if display_order_tasks.is_empty() {
            self.render_center_pane(
                ui,
                &all_tasks,
                display_order_tasks.len(),
                open_tasks,
                next_open_id,
            );
            return;
        }

        // Auto-selection logic
        self.handle_auto_selection(&display_order_tasks, next_open_id.clone());

        // Layout Calculations
        let available_rect = ui.available_rect_before_wrap();
        let available_height = available_rect.height();
        let available_width = available_rect.width();

        const MIN_CENTER_WIDTH: f32 = 450.0;
        const MIN_SIDEBAR_WIDTH: f32 = 220.0;
        let resize_handle_width = 8.0;

        // --- Left Panel Width ---
        let tree_width_id = ui.id().with("tree_panel_width");
        let saved_tree_width = ui
            .memory(|mem| mem.data.get_temp::<f32>(tree_width_id))
            .unwrap_or(280.0);

        // Calculate available space for sidebars after reserving center width
        // We also reserve space for resize handles (approx 16px)
        let max_sidebar_budget =
            (available_width - MIN_CENTER_WIDTH - (resize_handle_width * 2.0)).max(0.0);

        // Prioritize Left Panel
        let tree_width = saved_tree_width.clamp(MIN_SIDEBAR_WIDTH, max_sidebar_budget);

        // Determine actual visibility/width of Left Panel
        // If budget is tiny, we might hide it or squeeze center.
        // Strategy: Always show Left if at least MIN_SIDEBAR_WIDTH fits in budget.
        let safe_tree_width = if max_sidebar_budget >= MIN_SIDEBAR_WIDTH {
            tree_width
        } else {
            // If super narrow, check if we can fit it by squeezing center?
            // For now, let's just clamp to 0 if we strictly enforce center min width.
            // Or maybe we allow center to shrink if window is very small.
            // Let's stick to the plan: Hide/Collapse if it doesn't fit.
            if available_width > MIN_SIDEBAR_WIDTH + 200.0 {
                // Allow center to go below min if needed
                tree_width.min(available_width - 200.0)
            } else {
                0.0
            }
        };

        // --- Right Panel Width ---
        let threads_width_id = ui.id().with("threads_panel_width");
        let saved_threads_width = ui
            .memory(|mem| mem.data.get_temp::<f32>(threads_width_id))
            .unwrap_or(300.0);

        // Calculate remaining budget for Right Panel
        let remaining_for_right = (max_sidebar_budget - safe_tree_width).max(0.0);

        // Auto-hide Right Panel if not enough space
        let safe_threads_width = if remaining_for_right >= MIN_SIDEBAR_WIDTH {
            saved_threads_width.clamp(MIN_SIDEBAR_WIDTH, remaining_for_right)
        } else {
            0.0
        };

        // --- Rect Definitions ---
        let left_rect = egui::Rect::from_min_size(
            available_rect.min,
            egui::vec2(safe_tree_width, available_height),
        );
        let left_resize_rect = egui::Rect::from_min_size(
            egui::pos2(left_rect.max.x, left_rect.min.y - 6.0),
            egui::vec2(resize_handle_width, available_height + 6.0),
        );

        let right_rect = egui::Rect::from_min_size(
            egui::pos2(
                available_rect.max.x - safe_threads_width,
                available_rect.min.y,
            ),
            egui::vec2(safe_threads_width, available_height),
        );
        let right_resize_rect = egui::Rect::from_min_size(
            egui::pos2(
                right_rect.min.x - resize_handle_width,
                right_rect.min.y - 6.0,
            ),
            egui::vec2(resize_handle_width, available_height + 6.0),
        );

        // Center Panel Rect
        let center_min_x = left_resize_rect.max.x;
        let center_max_x = right_resize_rect.min.x;

        let center_rect = egui::Rect::from_min_max(
            egui::pos2(center_min_x, available_rect.min.y),
            egui::pos2(center_max_x.max(center_min_x), available_rect.max.y),
        );

        // Min viable width check for rendering content (sanity check)
        let min_viable_width = spacing::SPACING_MD * 2.0;

        // --- A. Left Panel (Navigation Tree) ---
        if safe_tree_width > min_viable_width {
            let mut left_ui = ui.new_child(egui::UiBuilder::new().max_rect(left_rect));
            egui::Frame::NONE
                .inner_margin(egui::Margin {
                    left: (spacing::SPACING_SM + 2.0) as i8,
                    right: 0,
                    top: spacing::SPACING_MD as i8,
                    bottom: spacing::SPACING_MD as i8,
                })
                .show(&mut left_ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    egui::ScrollArea::vertical()
                        .id_salt("nav_tree_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            let is_generating_this = self.state.session.is_generating
                                && self.state.ui.selected_review_id
                                    == self.state.session.generating_review_id;
                            if let Some(action) = render_navigation_tree(
                                ui,
                                &tasks_by_sub_flow,
                                self.state.ui.selected_task_id.as_ref(),
                                is_generating_this,
                                &theme,
                            ) {
                                self.dispatch(Action::Review(action));
                            }
                        });
                });
        }

        // --- Left Resize Handle ---
        let resize_response = ui.interact(
            left_resize_rect,
            ui.id().with("resize_tree"),
            egui::Sense::drag(),
        );
        if resize_response.dragged()
            && let Some(pointer_pos) = ui.ctx().pointer_interact_pos()
        {
            let new_width = pointer_pos.x - available_rect.min.x;
            ui.memory_mut(|mem| mem.data.insert_temp(tree_width_id, new_width));
        }

        // Draw Left Resize Visuals
        let hover_active = resize_response.hovered() || resize_response.dragged();
        if hover_active {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        let line_x = left_resize_rect.center().x + 3.5;
        let line_color = if hover_active {
            theme.accent
        } else {
            theme.border
        };
        ui.painter().line_segment(
            [
                egui::pos2(line_x, left_resize_rect.min.y),
                egui::pos2(line_x, left_resize_rect.max.y),
            ],
            egui::Stroke::new(1.0, line_color),
        );

        // --- B. Center Panel (Task Detail or Status) ---
        {
            let mut center_ui = ui.new_child(egui::UiBuilder::new().max_rect(center_rect));
            egui::Frame::NONE
                .fill(theme.bg_primary)
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

        // --- C. Right Resize Handle ---
        let right_resize_response = ui.interact(
            right_resize_rect,
            ui.id().with("resize_threads"),
            egui::Sense::drag(),
        );
        if right_resize_response.dragged()
            && let Some(pointer_pos) = ui.ctx().pointer_interact_pos()
        {
            // Dragging left increases width (since it's anchored right)
            let new_width = available_rect.max.x - pointer_pos.x;
            ui.memory_mut(|mem| mem.data.insert_temp(threads_width_id, new_width));
        }

        let right_hover_active = right_resize_response.hovered() || right_resize_response.dragged();
        if right_hover_active {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        let right_line_x = right_resize_rect.center().x - 3.5;
        let right_line_color = if right_hover_active {
            theme.accent
        } else {
            theme.border
        };
        ui.painter().line_segment(
            [
                egui::pos2(right_line_x, right_resize_rect.min.y),
                egui::pos2(right_line_x, right_resize_rect.max.y),
            ],
            egui::Stroke::new(1.0, right_line_color),
        );

        // --- D. Right Panel (Thread List) ---
        if safe_threads_width > min_viable_width {
            let mut right_ui = ui.new_child(egui::UiBuilder::new().max_rect(right_rect));
            egui::Frame::NONE
                .inner_margin(MarginF32 {
                    left: 0.0,
                    right: 0.0,
                    top: spacing::SPACING_LG,
                    bottom: spacing::SPACING_SM,
                })
                .show(&mut right_ui, |ui| {
                    egui::Frame::NONE
                        .inner_margin(Margin::symmetric(spacing::SPACING_SM as i8, 0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    typography::bold_label("All Threads").color(theme.text_primary),
                                );

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |_ui| {
                                        // Removed thread selection toggle button
                                    },
                                );
                            });
                        });

                    ui.add_space(spacing::SPACING_SM);
                    if let Some(review_id) = &self.state.ui.selected_review_id {
                        // Filter threads for current review
                        let review_threads: Vec<_> = self
                            .state
                            .domain
                            .threads
                            .iter()
                            .filter(|t| &t.review_id == review_id)
                            .cloned()
                            .collect();

                        let active_thread_id = self
                            .state
                            .ui
                            .active_thread
                            .as_ref()
                            .and_then(|t| t.thread_id.as_deref());

                        if let Some(action) =
                            crate::ui::views::review::thread_list::render_thread_list(
                                ui,
                                &review_threads,
                                active_thread_id,
                                false,
                                &std::collections::HashSet::new(),
                                true,
                                &theme,
                            )
                        {
                            self.dispatch(Action::Review(action));
                        }
                    }
                });
        }
    }

    fn handle_auto_selection(
        &mut self,
        _display_tasks: &[&crate::domain::ReviewTask],
        _next_open_id: Option<String>,
    ) {
        // Only enforce logic if selection is invalid or missing when it shouldn't be
        let selection_valid = self
            .state
            .ui
            .selected_task_id
            .as_ref()
            .is_some_and(|id| self.state.domain.all_tasks.iter().any(|t| &t.id == id));

        if !selection_valid && !self.state.ui.is_exporting {
            // Logic: If user was doing something, clear it. If just starting, maybe wait?
            // Current logic: clear if invalid.
            if self.state.ui.selected_task_id.is_some() {
                self.dispatch(Action::Review(ReviewAction::ClearSelection));
            }
        }
    }
}
