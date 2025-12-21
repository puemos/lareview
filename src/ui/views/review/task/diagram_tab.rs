use crate::ui::app::LaReviewApp;
use crate::ui::spacing;
use eframe::egui;

impl LaReviewApp {
    pub(crate) fn render_diagram_tab(
        &mut self,
        ui: &mut egui::Ui,
        task: &crate::domain::ReviewTask,
    ) {
        egui::Frame::NONE
            .inner_margin(spacing::SPACING_XL)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_min_height(400.0);
                    let go_to_settings = crate::ui::components::diagram::diagram_view(
                        ui,
                        &task.diagram,
                        ui.visuals().dark_mode,
                    );
                    if go_to_settings {
                        self.switch_to_settings();
                    }
                });
            });
    }
}
