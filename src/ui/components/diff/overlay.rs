use crate::ui::app::{Action, FullDiffView, LaReviewApp, ReviewAction};
use crate::ui::components::DiffAction;
use crate::ui::{icons, spacing};
use eframe::egui;

pub fn render(ctx: &egui::Context, app: &mut LaReviewApp, full: &FullDiffView) {
    let viewport_rect = ctx.input(|i| i.viewport().inner_rect).unwrap_or_else(|| {
        let rect = ctx.available_rect();
        egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), rect.size())
    });

    if viewport_rect.width() < 100.0 || viewport_rect.height() < 100.0 {
        return;
    }

    let mut open = true;
    let outer_padding = egui::vec2(spacing::SPACING_MD, spacing::SPACING_SM);

    egui::Window::new(full.title.clone())
        .id(egui::Id::new("full_diff_overlay"))
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
                if ui.button(format!("{} Close", icons::ACTION_BACK)).clicked() {
                    app.dispatch(Action::Review(ReviewAction::CloseFullDiff));
                }
            });

            ui.separator();

            let action =
                crate::ui::components::diff::render_diff_editor_full_view(ui, &full.text, "diff");
            if let DiffAction::OpenInEditor {
                file_path,
                line_number,
            } = action
            {
                app.dispatch(Action::Review(ReviewAction::OpenInEditor {
                    file_path,
                    line_number,
                }));
            }
        });

    if !open {
        app.dispatch(Action::Review(ReviewAction::CloseFullDiff));
    }
}
