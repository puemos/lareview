use crate::ui::app::LaReviewApp;
use crate::ui::components::action_button::action_button;
use crate::ui::components::pills::pill_action_button;
use crate::ui::spacing;
use crate::ui::theme::current_theme;
use eframe::egui;
use egui_phosphor::regular as icons;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RightPaneState {
    TaskSelected,
    ReadyNoSelection,
    NoTasks,
    AllDone,
}

impl LaReviewApp {
    /// Renders the logic for the Center Panel
    pub(super) fn render_center_pane(
        &mut self,
        ui: &mut egui::Ui,
        all_tasks: &[crate::domain::ReviewTask],
        total_count: usize,
        open_count: usize,
        next_open_id: Option<String>,
    ) {
        let state = if self.state.ui.selected_task_id.is_some() {
            // Validate selection still exists
            if all_tasks
                .iter()
                .any(|t| Some(&t.id) == self.state.ui.selected_task_id.as_ref())
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
                if let Some(task_id) = &self.state.ui.selected_task_id
                    && let Some(task) = all_tasks.iter().find(|t| &t.id == task_id)
                {
                    self.render_task_detail(ui, task);
                }
            }
            RightPaneState::ReadyNoSelection => {
                self.render_ready_state(ui, next_open_id);
            }
            RightPaneState::AllDone => {
                let min_width = spacing::SPACING_XL * 2.0 + 10.0;
                if ui.available_width() < min_width {
                    return;
                }
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
    pub(super) fn render_ready_state(&mut self, ui: &mut egui::Ui, next_open_id: Option<String>) {
        // Safety: ensure enough width for margins
        let min_width = spacing::SPACING_XL * 2.0 + 10.0;
        if ui.available_width() < min_width {
            return;
        }

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

    /// Renders the "No Tasks" empty state
    pub(super) fn render_empty_state(&mut self, ui: &mut egui::Ui) {
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
}
