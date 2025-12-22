use crate::domain::ReviewTask;
use crate::ui::app::{Action, LaReviewApp, ReviewAction};
use crate::ui::components::action_button::action_button;
use crate::ui::components::pills::pill_action_button;
use crate::ui::spacing;
use crate::ui::theme::Theme;
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
        all_tasks: &[ReviewTask],
        total_count: usize,
        open_count: usize,
        next_open_id: Option<String>,
    ) {
        let theme = crate::ui::theme::current_theme();
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
                if let Some(action) = render_ready_state(ui, next_open_id, &theme) {
                    self.dispatch(Action::Review(action));
                }
            }
            RightPaneState::AllDone => {
                render_all_done_state(ui, &theme);
            }
            RightPaneState::NoTasks => {
                if let Some(action) = render_empty_state(ui, &theme) {
                    self.dispatch(action);
                }
            }
        }
    }
}

pub(crate) fn render_all_done_state(ui: &mut egui::Ui, theme: &Theme) {
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
                        .color(theme.success),
                );
                ui.add_space(16.0);
                ui.heading("All tasks completed!");
                ui.label(egui::RichText::new("Great job.").color(theme.text_muted));
            });
        });
}

/// Screen shown when tasks exist but none are selected
pub(crate) fn render_ready_state(
    ui: &mut egui::Ui,
    next_open_id: Option<String>,
    theme: &Theme,
) -> Option<ReviewAction> {
    // Safety: ensure enough width for margins
    let min_width = spacing::SPACING_XL * 2.0 + 10.0;
    if ui.available_width() < min_width {
        return None;
    }

    // Keyboard shortcuts logic
    let mut trigger_primary = false;
    if ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift) {
        trigger_primary = true;
    }

    let mut action_out = None;

    egui::Frame::NONE
        .inner_margin(spacing::SPACING_XL)
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.25);

                // Hero Icon
                ui.label(
                    egui::RichText::new(icons::LIST_CHECKS)
                        .size(64.0)
                        .color(theme.brand.gamma_multiply(0.8)),
                );
                ui.add_space(24.0);

                ui.heading("Ready to Review");
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Select a task from the sidebar or start the queue.")
                        .color(theme.text_secondary),
                );

                ui.add_space(32.0);

                // Primary Action
                let btn_enabled = next_open_id.is_some();
                let resp = pill_action_button(
                    ui,
                    icons::ARROW_RIGHT,
                    "Start Reviewing",
                    btn_enabled,
                    theme.brand,
                );

                // Hint for keyboard shortcut
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Press [Enter] to start")
                        .size(10.0)
                        .color(theme.text_disabled),
                );

                if (resp.clicked() || trigger_primary)
                    && btn_enabled
                    && let Some(id) = next_open_id
                {
                    action_out = Some(ReviewAction::SelectTask { task_id: id });
                }
            });
        });

    action_out
}

/// Renders the "No Tasks" empty state
pub(crate) fn render_empty_state(ui: &mut egui::Ui, theme: &Theme) -> Option<Action> {
    let mut action_out = None;
    ui.allocate_ui_with_layout(
        ui.available_size(),
        egui::Layout::centered_and_justified(egui::Direction::TopDown),
        |ui| {
            ui.vertical_centered(|ui| {
                // Hero Icon
                ui.label(
                    egui::RichText::new(icons::BOUNDING_BOX)
                        .size(64.0)
                        .color(theme.border_secondary),
                );
                ui.add_space(spacing::SPACING_MD);
                ui.heading("No review tasks yet");
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Generate tasks from your diff to start reviewing.")
                        .color(theme.text_muted),
                );
                ui.add_space(24.0);

                if action_button(ui, "Generate tasks", true, theme.brand).clicked() {
                    action_out = Some(Action::Navigation(
                        crate::ui::app::NavigationAction::SwitchTo(
                            crate::ui::app::AppView::Generate,
                        ),
                    ));
                }
            });
        },
    );
    action_out
}
