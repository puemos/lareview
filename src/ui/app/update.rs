use eframe::egui;

use super::LaReviewApp;
use super::state::AppView;

impl eframe::App for LaReviewApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Set the base Catppuccin theme for overall UI appearance
        catppuccin_egui::set_theme(ctx, catppuccin_egui::MOCHA);

        self.poll_gh_messages();
        self.poll_d2_install_messages();
        let action_updated = self.poll_action_messages();
        let agent_content_updated = self.poll_generation_messages();

        if action_updated
            || agent_content_updated
            || self.state.is_generating
            || self.state.is_exporting
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        self.render_header(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| match self.state.current_view {
                    AppView::Generate => self.ui_generate(ui),
                    AppView::Review => self.ui_review(ui),
                    AppView::Settings => self.ui_settings(ui),
                });
        });

        self.render_full_diff_overlay(ctx);
        self.render_export_preview_overlay(ctx);
    }
}
