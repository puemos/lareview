use eframe::egui;
use egui_phosphor::regular as icons;

use super::LaReviewApp;
use super::state::AppView;
use crate::ui::spacing::SPACING_MD;
use crate::ui::theme;

impl LaReviewApp {
    pub(super) fn render_header(&mut self, ctx: &egui::Context) {
        let theme = theme::current_theme();

        // 1. Define styling constants for a "Pro" feel
        let header_height = 52.0;
        let nav_rounding = 6.0;

        egui::TopBottomPanel::top("header")
            .exact_height(header_height)
            .show(ctx, |ui| {
                // Setup layout for 3 columns: Left (Logo), Center (Nav), Right (actions)
                let rect = ui.available_rect_before_wrap();

                // --- 1. LEFT: App Identity ---
                // We use allocate_ui_at_rect to pin this to the left
                let left_rect =
                    egui::Rect::from_min_size(rect.min, egui::vec2(200.0, rect.height()));
                ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect), |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.add_space(SPACING_MD); // SPACE FOR MACOS TRAFFIC LIGHTS

                        // Just the icon, slightly larger
                        ui.label(
                            egui::RichText::new(icons::CIRCLE_HALF)
                                .size(22.0)
                                .color(theme.brand),
                        );

                        ui.add_space(4.0);

                        // App Name - slightly muted to let content shine
                        ui.label(
                            egui::RichText::new("LaReview")
                                .strong()
                                .size(14.0)
                                .color(theme.text_primary),
                        );
                    });
                });

                // --- 2. CENTER: Navigation Tabs ---
                // Absolute center positioning looks best for tools
                let center_width = 380.0;
                let center_rect =
                    egui::Rect::from_center_size(rect.center(), egui::vec2(center_width, 32.0));
                // Slightly inset the tabs from the pill background to give breathing room
                let tab_rect = center_rect.shrink2(egui::vec2(6.0, 3.0));

                ui.scope_builder(egui::UiBuilder::new().max_rect(tab_rect), |ui| {
                    // Draw a background container for the tabs (Segmented Control style)
                    ui.painter().rect_filled(
                        center_rect,
                        egui::CornerRadius::same(nav_rounding as u8),
                        theme.bg_secondary, // Darker background for the pill container
                    );

                    ui.horizontal(|ui| {
                        let spacing = 4.0;
                        ui.spacing_mut().item_spacing = egui::vec2(spacing, 0.0);

                        // We split the width equally among tabs, accounting for spacing
                        let tab_width = (tab_rect.width() - (3.0 * spacing)) / 4.0;

                        self.render_tab(
                            ui,
                            AppView::Generate,
                            "Generate",
                            icons::GIT_DIFF,
                            tab_width,
                            nav_rounding,
                        );
                        self.render_tab(
                            ui,
                            AppView::Review,
                            "Review",
                            icons::COFFEE,
                            tab_width,
                            nav_rounding,
                        );
                        self.render_tab(
                            ui,
                            AppView::Repos,
                            "Repos",
                            icons::GIT_BRANCH,
                            tab_width,
                            nav_rounding,
                        );
                        self.render_tab(
                            ui,
                            AppView::Settings,
                            "Settings",
                            icons::GEAR,
                            tab_width,
                            nav_rounding,
                        );
                    });
                });
            });
    }

    // Helper for the "Segmented Control" style tabs
    fn render_tab(
        &mut self,
        ui: &mut egui::Ui,
        view: AppView,
        label: &str,
        icon: &str,
        width: f32,
        rounding: f32,
    ) {
        let is_active = self.state.current_view == view;
        let theme = theme::current_theme();

        let (bg, stroke) = if is_active {
            (theme.bg_primary, egui::Stroke::new(1.0, theme.border))
        } else {
            (
                egui::Color32::TRANSPARENT,
                egui::Stroke::new(1.0, egui::Color32::TRANSPARENT),
            )
        };

        let mut text = egui::RichText::new(format!("{} {}", icon, label))
            .size(13.0)
            .strong();

        if is_active {
            text = text.color(theme.brand);
        }

        ui.scope(|ui| {
            let visuals = &mut ui.style_mut().visuals.widgets;
            visuals.inactive.fg_stroke.color = theme.text_secondary;
            visuals.hovered.fg_stroke.color = theme.text_primary;

            let btn = egui::Button::new(text)
                .fill(bg)
                .stroke(stroke)
                .corner_radius(rounding - 2.0) // Slightly less rounding than container
                .min_size(egui::vec2(width, 26.0)); // Fill the segment while leaving pill padding

            if ui.add(btn).clicked() {
                match view {
                    AppView::Generate => self.switch_to_generate(),
                    AppView::Review => self.switch_to_review(),
                    AppView::Repos => self.switch_to_repos(),
                    AppView::Settings => self.switch_to_settings(),
                }
            }
        });
    }
}
