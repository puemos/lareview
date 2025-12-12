use eframe::egui;

use super::LaReviewApp;

impl LaReviewApp {
    pub(super) fn render_full_diff_overlay(&mut self, ctx: &egui::Context) {
        let Some(full) = self.state.full_diff.clone() else {
            return;
        };

        let viewport_rect = ctx.input(|i| i.viewport().inner_rect).unwrap_or_else(|| {
            let rect = ctx.available_rect();
            egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), rect.size())
        });

        let mut open = true;
        let outer_padding = egui::vec2(12.0, 8.0);

        egui::Window::new(full.title.clone())
            .open(&mut open)
            .fixed_rect(viewport_rect.shrink2(outer_padding))
            .frame(egui::Frame::window(&ctx.style()).inner_margin(egui::Margin::symmetric(12, 8)))
            .collapsible(false)
            .resizable(false)
            .title_bar(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(format!("{} Close", egui_phosphor::regular::ARROW_SQUARE_IN))
                        .clicked()
                    {
                        self.state.full_diff = None;
                    }
                });

                ui.separator();

                crate::ui::components::diff::render_diff_editor_full_view(ui, &full.text, "diff");
            });

        if !open {
            self.state.full_diff = None;
        }
    }
}
