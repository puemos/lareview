//! Review screen (task list + task detail)

use crate::application::review::ordering::tasks_in_display_order;
use crate::ui::app::{Action, LaReviewApp, ReviewAction};
use crate::ui::components::pills::pill_action_button;
use crate::ui::components::status::error_banner;
use crate::ui::spacing;
use crate::ui::theme::current_theme;
use eframe::egui;
use egui_phosphor::regular as icons;

use crate::ui::views::review::nav::render_navigation_tree;
use crate::ui::views::review::toolbar::render_header_selectors;

// Optimized header height for a "Toolbar" feel
const TOP_HEADER_HEIGHT: f32 = 52.0;

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
                            if let Some(action) = render_header_selectors(
                                ui,
                                &self.state.domain.reviews,
                                self.state.ui.selected_review_id.as_ref(),
                                &theme,
                            ) {
                                self.dispatch(Action::Review(action));
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
                                    icons::TRASH_SIMPLE,
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
                                    icons::EXPORT,
                                    "Export",
                                    review_selected,
                                    theme.border,
                                )
                                .on_hover_text("Export as markdown")
                                .clicked()
                                {
                                    self.dispatch(Action::Review(
                                        ReviewAction::RequestExportPreview,
                                    ));
                                }

                                if self.state.ui.is_exporting {
                                    ui.add_space(spacing::SPACING_XS);
                                    crate::ui::animations::cyber::cyber_spinner(ui, theme.brand);
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

        // Draw Full-Width Separator
        let bar_rect = egui::Rect::from_min_size(
            egui::pos2(
                content_rect.left(),
                header_response.response.rect.bottom() + 2.0,
            ),
            egui::vec2(content_rect.width(), 1.0),
        );
        ui.painter().rect_filled(bar_rect, 0.0, theme.border);

        // Handle delayed actions
        if trigger_delete_review {
            self.dispatch(Action::Review(ReviewAction::DeleteReview));
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

        let tree_width_id = ui.id().with("tree_panel_width");
        let saved_tree_width = ui
            .memory(|mem| mem.data.get_temp::<f32>(tree_width_id))
            .unwrap_or(280.0);
        let max_tree_width = (available_width - 100.0).max(0.0); // Leave at least 100px for center
        let tree_width = crate::ui::layout::clamp_width(saved_tree_width, 220.0, max_tree_width);

        let resize_handle_width = 8.0;

        // Rect Definitions
        // Minimum width to avoid negative size panics (2 * SPACING_MD + some buffer)
        let min_viable_width = spacing::SPACING_MD * 2.0 + 10.0;
        let safe_tree_width = if available_width > min_viable_width + 50.0 {
            tree_width.min(available_width - 50.0).max(0.0)
        } else {
            0.0
        };

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
        if safe_tree_width > min_viable_width {
            let mut left_ui = ui.new_child(egui::UiBuilder::new().max_rect(left_rect));
            // Slight contrast for navigation panel
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
                            if let Some(action) = render_navigation_tree(
                                ui,
                                &tasks_by_sub_flow,
                                self.state.ui.selected_task_id.as_ref(),
                                &theme,
                            ) {
                                self.dispatch(Action::Review(action));
                            }
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
            theme.accent
        } else {
            theme.border
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
