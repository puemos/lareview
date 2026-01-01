use crate::ui::app::{Action, LaReviewApp};
use crate::ui::spacing;
use crate::ui::theme::current_theme;
use crate::ui::views::review::task::{ReviewTab, render_task_header, render_task_tabs};
use eframe::egui;
use egui::epaint::MarginF32;

impl LaReviewApp {
    /// Renders the detailed view of the selected task
    pub(super) fn render_task_detail(
        &mut self,
        ui: &mut egui::Ui,
        task: &crate::domain::ReviewTask,
    ) {
        let min_width = spacing::SPACING_XL * 2.0 + 10.0;
        if ui.available_width() < min_width {
            return;
        }

        let theme = current_theme();

        egui::Frame::NONE
            .inner_margin(MarginF32 {
                left: spacing::SPACING_XL,
                right: spacing::SPACING_XL,
                top: spacing::SPACING_XL,
                bottom: spacing::SPACING_SM,
            })
            .show(ui, |ui| {
                if let Some(action) = render_task_header(ui, task, &theme) {
                    self.dispatch(Action::Review(action));
                }

                ui.add_space(spacing::SPACING_LG);

                let (_active_tab, action) =
                    render_task_tabs(ui, task, &self.state.ui, &self.state.domain, &theme);
                if let Some(action) = action {
                    self.dispatch(Action::Review(action));
                }
            });

        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt(format!("detail_scroll_{}", task.id))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut active_tab = ui
                    .ctx()
                    .data(|d| d.get_temp::<ReviewTab>(egui::Id::new(("active_tab", &task.id))))
                    .unwrap_or(ReviewTab::Description);

                if self.state.ui.active_feedback.is_some() {
                    active_tab = ReviewTab::Feedback;
                }

                match active_tab {
                    ReviewTab::Description => self.render_description_tab(ui, task),
                    ReviewTab::Diagram => self.render_diagram_tab(ui, task),
                    ReviewTab::Changes => self.render_changes_tab(ui, task),
                    ReviewTab::Feedback => self.render_feedback_tab(ui, task),
                }
            });
    }
}
