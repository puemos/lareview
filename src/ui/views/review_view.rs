use crate::ui::app::LaReviewApp;
use crate::ui::components::header::action_button;
use eframe::egui;

// diff editor helper
use crate::ui::components::status::error_banner;
use crate::ui::components::{DiffAction, LineContext};
use catppuccin_egui::MOCHA;
use egui_phosphor::regular as icons;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RightPaneState {
    TaskSelected,
    ReadyNoSelection,
    NoTasks,
    AllDone,
}

/// Combines multiple diff strings into a single unified diff string
fn combine_diffs_to_unified_diff(diffs: &[String]) -> String {
    // For now, we just join the diff strings with newlines
    // The Patch struct was removed, so diffs are just raw strings
    diffs.join("\n")
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
            self.state.selected_task_id = None;
            self.state.current_note = None;
            self.state.current_line_note = None;
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
                self.state.selected_task_id = None;
                self.state.current_note = None;
                self.state.current_line_note = None;
            }
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
                                .filter(|t| {
                                    matches!(
                                        t.status,
                                        crate::domain::TaskStatus::Done
                                            | crate::domain::TaskStatus::Ignored
                                    )
                                })
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
                                    let mut tasks_sorted: Vec<_> = tasks.iter().collect();
                                    tasks_sorted.sort_by_key(|t| {
                                        let is_closed = matches!(
                                            t.status,
                                            crate::domain::TaskStatus::Done
                                                | crate::domain::TaskStatus::Ignored
                                        );
                                        (
                                            is_closed,
                                            std::cmp::Reverse(risk_rank(t.stats.risk)),
                                            t.title.as_str(),
                                        )
                                    });
                                    for task in tasks_sorted {
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
                        self.state.selected_task_id = None;
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

    /// Renders a single task item in the sidebar
    /// Renders a single task item in the sidebar
    fn render_nav_item(&mut self, ui: &mut egui::Ui, task: &crate::domain::ReviewTask) {
        let is_selected = self.state.selected_task_id.as_ref() == Some(&task.id);

        let (bg_color, text_color) = if is_selected {
            (MOCHA.surface1, MOCHA.text)
        } else {
            (egui::Color32::TRANSPARENT, MOCHA.subtext0)
        };

        let (risk_icon, risk_color, risk_label) = match task.stats.risk {
            crate::domain::RiskLevel::High => {
                (icons::CARET_CIRCLE_DOUBLE_UP, MOCHA.red, "High risk")
            }
            crate::domain::RiskLevel::Medium => {
                (icons::CARET_CIRCLE_UP, MOCHA.yellow, "Medium risk")
            }
            crate::domain::RiskLevel::Low => (icons::CARET_CIRCLE_DOWN, MOCHA.blue, "Low risk"),
        };

        let is_closed = matches!(
            task.status,
            crate::domain::TaskStatus::Done | crate::domain::TaskStatus::Ignored
        );

        let mut title_text = egui::RichText::new(&task.title)
            .size(13.0)
            .color(text_color);
        if is_closed {
            title_text = title_text.color(MOCHA.subtext0).strikethrough();
        }

        let response = egui::Frame::NONE
            .fill(bg_color)
            .corner_radius(4.0)
            .inner_margin(4.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    // Navigation: risk + crossed title (when closed)
                    ui.label(egui::RichText::new(risk_icon).size(16.0).color(risk_color))
                        .on_hover_text(risk_label);

                    ui.add_space(6.0);

                    ui.add(egui::Label::new(title_text).wrap());
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
            self.select_task(task);
        }
    }

    fn select_task(&mut self, task: &crate::domain::ReviewTask) {
        self.state.selected_task_id = Some(task.id.clone());
        self.state.current_line_note = None;

        if let Ok(Some(note)) = self.note_repo.find_by_task(&task.id) {
            self.state.current_note = Some(note.body);
        } else {
            self.state.current_note = Some(String::new());
        }
    }

    fn select_task_by_id(&mut self, all_tasks: &[crate::domain::ReviewTask], task_id: &str) {
        if let Some(task) = all_tasks.iter().find(|t| t.id == task_id) {
            self.select_task(task);
        }
    }

    /// Renders the detailed view of the selected task
    fn render_task_detail(&mut self, ui: &mut egui::Ui, task: &crate::domain::ReviewTask) {
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

                    let to_do_resp = status_chip(
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

                    let in_progress_resp = status_chip(
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

                    let done_resp = status_chip(
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

                    let ignored_resp = status_chip(
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

fn tasks_in_display_order(
    tasks_by_sub_flow: &std::collections::HashMap<
        Option<String>,
        Vec<crate::domain::ReviewTask>,
    >,
) -> Vec<&crate::domain::ReviewTask> {
    let mut sub_flows: Vec<_> = tasks_by_sub_flow.iter().collect();
    sub_flows.sort_by(|(name_a, _), (name_b, _)| {
        name_a
            .as_deref()
            .unwrap_or("ZZZ")
            .cmp(name_b.as_deref().unwrap_or("ZZZ"))
    });

    let mut out = Vec::new();
    for (_sub_flow_name, tasks) in sub_flows {
        let mut tasks_sorted: Vec<_> = tasks.iter().collect();
        tasks_sorted.sort_by_key(|t| {
            let is_closed = matches!(
                t.status,
                crate::domain::TaskStatus::Done | crate::domain::TaskStatus::Ignored
            );
            (
                is_closed,
                std::cmp::Reverse(risk_rank(t.stats.risk)),
                t.title.as_str(),
            )
        });
        out.extend(tasks_sorted);
    }
    out
}

fn risk_rank(risk: crate::domain::RiskLevel) -> u8 {
    match risk {
        crate::domain::RiskLevel::High => 2,
        crate::domain::RiskLevel::Medium => 1,
        crate::domain::RiskLevel::Low => 0,
    }
}

// Extract file path from diff string
fn extract_file_path_from_diff(diff: &str) -> Option<String> {
    for line in diff.lines() {
        if line.starts_with("diff --git ") {
            // Extract file path from the diff line
            // Format: "diff --git a/path/file b/path/file"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let file_part = parts[1]; // This is a/path/file
                if let Some(stripped) = file_part.strip_prefix("a/") {
                    return Some(stripped.to_string());
                }
            }
        }
    }
    None
}

fn pill_divider(ui: &mut egui::Ui) {
    ui.add_sized(
        egui::vec2(6.0, 22.0),
        egui::Separator::default().vertical(),
    );
}

fn pill_action_button(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    enabled: bool,
    tint: egui::Color32,
) -> egui::Response {
    let text = egui::RichText::new(format!("{icon} {label}"))
        .size(12.0)
        .color(if enabled { MOCHA.text } else { MOCHA.subtext0 });

    let fill = if enabled {
        MOCHA.surface0
    } else {
        MOCHA.mantle
    };

    let stroke = if enabled { tint } else { MOCHA.surface2 };

    let old_padding = ui.spacing().button_padding;
    ui.spacing_mut().button_padding = egui::vec2(8.0, 4.0);

    let resp = ui.add_enabled(
        enabled,
        egui::Button::new(text)
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, stroke))
            .corner_radius(egui::CornerRadius::same(255))
            .min_size(egui::vec2(0.0, 24.0)),
    );

    ui.spacing_mut().button_padding = old_padding;
    resp
}

fn status_chip(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    selected: bool,
    tint: egui::Color32,
) -> egui::Response {
    let (fill, stroke, fg) = if selected {
        (tint.gamma_multiply(0.18), tint, tint)
    } else {
        (MOCHA.surface0, MOCHA.surface2, MOCHA.subtext0)
    };

    let text = egui::RichText::new(format!("{icon} {label}"))
        .size(10.0)
        .color(fg);

    let old_padding = ui.spacing().button_padding;
    ui.spacing_mut().button_padding = egui::vec2(8.0, 4.0);

    let resp = ui.add(
        egui::Button::new(text)
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, stroke))
            .corner_radius(egui::CornerRadius::same(255))
            .min_size(egui::vec2(0.0, 22.0)),
    );

    ui.spacing_mut().button_padding = old_padding;
    resp
}

// Helper for drawing badges
fn badge(ui: &mut egui::Ui, text: &str, bg: egui::Color32, fg: egui::Color32) {
    egui::Frame::NONE
        .fill(bg)
        .stroke(egui::Stroke::new(1.0, MOCHA.surface2))
        .corner_radius(egui::CornerRadius::same(255))
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).size(10.0).color(fg));
        });
}
