use eframe::egui;

use super::{Action, LaReviewApp, ReviewAction};
use crate::ui::spacing;

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
        let outer_padding = egui::vec2(spacing::SPACING_MD, spacing::SPACING_SM);

        egui::Window::new(full.title.clone())
            .open(&mut open)
            .fixed_rect(viewport_rect.shrink2(outer_padding))
            .frame(
                egui::Frame::window(&ctx.style()).inner_margin(egui::Margin::symmetric(
                    spacing::SPACING_MD as i8,
                    spacing::SPACING_SM as i8,
                )),
            )
            .collapsible(false)
            .resizable(false)
            .title_bar(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(format!("{} Close", egui_phosphor::regular::ARROW_SQUARE_IN))
                        .clicked()
                    {
                        self.dispatch(Action::Review(ReviewAction::CloseFullDiff));
                    }
                });

                ui.separator();

                crate::ui::components::diff::render_diff_editor_full_view(ui, &full.text, "diff");
            });

        if !open {
            self.dispatch(Action::Review(ReviewAction::CloseFullDiff));
        }
    }
}
