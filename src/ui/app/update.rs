use eframe::egui;

use super::LaReviewApp;
use super::state::AppView;

impl eframe::App for LaReviewApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::ui::window::apply_rounded_corners(_frame);
        // Set the base Catppuccin theme for overall UI appearance
        catppuccin_egui::set_theme(ctx, catppuccin_egui::MOCHA);

        // Apply TUI-like visuals: disable corner rounding and set terminal backgrounds
        let theme = crate::ui::theme::current_theme();
        let mut visuals = egui::Visuals::dark();

        // Backgrounds
        visuals.panel_fill = theme.bg_primary;
        visuals.window_fill = theme.bg_primary;

        // Borders and Backgrounds for Widgets - TUI style (transparent bg, solid border)
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, theme.border);
        visuals.widgets.noninteractive.bg_fill = egui::Color32::TRANSPARENT;

        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, theme.border);
        visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;

        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, theme.brand);
        visuals.widgets.hovered.bg_fill = theme.bg_secondary; // Subtle lift on hover

        visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, theme.brand);
        visuals.widgets.active.bg_fill = theme.bg_secondary;

        visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, theme.border);
        visuals.widgets.open.bg_fill = theme.bg_primary;

        // Selection
        visuals.selection.bg_fill = theme.brand.gamma_multiply(0.3);
        visuals.selection.stroke = egui::Stroke::new(1.0, theme.brand);

        // Corner Radii
        visuals.window_corner_radius = egui::CornerRadius::same(crate::ui::spacing::RADIUS_LG);
        visuals.widgets.noninteractive.corner_radius =
            egui::CornerRadius::same(crate::ui::spacing::RADIUS_MD);
        visuals.widgets.inactive.corner_radius =
            egui::CornerRadius::same(crate::ui::spacing::RADIUS_MD);
        visuals.widgets.hovered.corner_radius =
            egui::CornerRadius::same(crate::ui::spacing::RADIUS_MD);
        visuals.widgets.active.corner_radius =
            egui::CornerRadius::same(crate::ui::spacing::RADIUS_MD);
        visuals.widgets.open.corner_radius =
            egui::CornerRadius::same(crate::ui::spacing::RADIUS_MD);

        ctx.set_visuals(visuals);

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
