use eframe::egui;

use super::LaReviewApp;
use super::state::AppView;

impl eframe::App for LaReviewApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::ui::window::apply_rounded_corners(_frame);
        self.render(ctx);
    }
}

impl LaReviewApp {
    pub fn render(&mut self, ctx: &egui::Context) {
        // Set the base Catppuccin theme for overall UI appearance
        catppuccin_egui::set_theme(ctx, catppuccin_egui::MOCHA);

        // Apply a high-contrast visual style inspired by terminal user interfaces (TUI).
        // This involves using solid borders, transparent widget backgrounds, and
        // the application's brand color for interactive states.
        let theme = crate::ui::theme::current_theme();
        let mut visuals = egui::Visuals::dark();

        // Configure panel and window backgrounds
        visuals.panel_fill = theme.bg_primary;
        visuals.window_fill = theme.bg_primary;

        // Configure widget aesthetics for a "flat" TUI look with distinct borders
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, theme.border);
        visuals.widgets.noninteractive.bg_fill = egui::Color32::TRANSPARENT;

        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, theme.border);
        visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;

        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, theme.brand);
        visuals.widgets.hovered.bg_fill = theme.bg_secondary; // Subtle elevation on hover

        visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, theme.brand);
        visuals.widgets.active.bg_fill = theme.bg_secondary;

        visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, theme.border);
        visuals.widgets.open.bg_fill = theme.bg_primary;

        // Selection styling
        visuals.selection.bg_fill = theme.brand.gamma_multiply(0.3);
        visuals.selection.stroke = egui::Stroke::new(1.0, theme.brand);

        // Configure standard corner radii for the application
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

        if action_updated || agent_content_updated {
            ctx.request_repaint();
        }

        let is_exporting = matches!(
            self.state.ui.active_overlay,
            Some(crate::ui::app::OverlayState::Export(ref data)) if data.is_exporting
        );
        if self.state.session.is_generating || is_exporting {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }

        if let Some(ref fatal_error) = self.state.ui.fatal_error {
            catppuccin_egui::set_theme(ctx, catppuccin_egui::MOCHA);
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.fill(theme.bg_primary))
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.heading(
                            egui::RichText::new("Fatal Initialization Error")
                                .color(theme.destructive)
                                .size(32.0),
                        );
                        ui.add_space(20.0);
                        ui.label(
                            egui::RichText::new(fatal_error)
                                .color(theme.text_primary)
                                .size(18.0),
                        );
                        ui.add_space(40.0);
                        if ui.button("Exit Application").clicked() {
                            std::process::exit(1);
                        }
                    });
                });
            return;
        }

        self.render_header(ctx);

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme.bg_primary))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| match self.state.ui.current_view {
                        AppView::Generate => self.ui_generate(ui),
                        AppView::Review => self.ui_review(ui),
                        AppView::Repos => self.ui_repos(ui),
                        AppView::Settings => self.ui_settings(ui),
                    });
            });

        self.render_overlays(ctx);

        if let Some(text) = self.state.ui.pending_clipboard_copy.take() {
            ctx.output_mut(|o| o.commands.push(egui::OutputCommand::CopyText(text)));
        }
    }
}
