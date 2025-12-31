use super::LaReviewApp;
use crate::ui::app::OverlayState;
use eframe::egui;

impl LaReviewApp {
    pub(super) fn render_overlays(&mut self, ctx: &egui::Context) {
        let Some(active) = self.state.ui.active_overlay.clone() else {
            return;
        };

        match active {
            OverlayState::FullDiff(full) => {
                crate::ui::components::diff::overlay::render(ctx, self, &full);
            }
            OverlayState::Export(data) => {
                crate::ui::views::review::overlays::export::render(ctx, self, &data);
            }
            OverlayState::PushFeedback(feedback_id) => {
                crate::ui::views::review::overlays::feedback::render_push_feedback_confirm(
                    ctx,
                    self,
                    &feedback_id,
                );
            }
            OverlayState::SendToPr(data) => {
                crate::ui::views::review::overlays::feedback::render_send_to_pr_overlay(
                    ctx, self, &data,
                );
            }
            OverlayState::Requirements => {
                crate::ui::views::settings_overlays::render_requirements_overlay(ctx, self);
            }
            OverlayState::EditorPicker => {
                crate::ui::views::settings_overlays::render_editor_picker_overlay(ctx, self);
            }
        }
    }
}
