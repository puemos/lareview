use crate::ui::app::{Action, LaReviewApp};
use crate::ui::spacing;
use crate::ui::theme::current_theme;
use crate::ui::views::review::task::{ReviewTab, render_task_header, render_task_tabs};
use eframe::egui;

impl LaReviewApp {
    /// Renders the detailed view of the selected task
    pub(super) fn render_task_detail(
        &mut self,
        ui: &mut egui::Ui,
        task: &crate::domain::ReviewTask,
    ) {
        // Safety: ensure enough width for margins
        let min_width = spacing::SPACING_XL * 2.0 + 10.0;
        if ui.available_width() < min_width {
            return;
        }

        let theme = current_theme();

        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(
                spacing::SPACING_XL as i8,
                spacing::SPACING_XS as i8,
            ))
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

        // 4. Content Area
        egui::ScrollArea::vertical()
            .id_salt(format!("detail_scroll_{}", task.id))
            .show(ui, |ui| {
                // Fetch active tab
                let mut active_tab = ui
                    .ctx()
                    .data(|d| d.get_temp::<ReviewTab>(egui::Id::new(("active_tab", &task.id))))
                    .unwrap_or(ReviewTab::Description);

                if self.state.ui.active_thread.is_some() {
                    active_tab = ReviewTab::Discussion;
                }

                match active_tab {
                    ReviewTab::Description => self.render_description_tab(ui, task),
                    ReviewTab::Diagram => self.render_diagram_tab(ui, task),
                    ReviewTab::Changes => self.render_changes_tab(ui, task),
                    ReviewTab::Discussion => self.render_discussion_tab(ui, task),
                }
            });
    }
}
