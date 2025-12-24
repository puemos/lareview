use crate::ui::app::AppView;
use crate::ui::app::LaReviewApp;
use crate::ui::icons;
use crate::ui::spacing;
use crate::ui::{theme, typography};
use eframe::egui;

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
                let left_rect =
                    egui::Rect::from_min_size(rect.min, egui::vec2(200.0, rect.height()));
                ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect), |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.add_space(spacing::SPACING_SM); // SPACE FOR MACOS TRAFFIC LIGHTS

                        // Just the icon, slightly larger
                        ui.label(
                            egui::RichText::new(icons::STATUS_IN_PROGRESS)
                                .size(22.0)
                                .color(theme.brand),
                        );

                        ui.add_space(2.0);

                        // App Name - slightly muted to let content shine
                        ui.label(
                            typography::bold("LaReview")
                                .size(14.0)
                                .color(theme.text_primary),
                        );
                    });
                });

                // --- 2. CENTER: Navigation Tabs ---
                let spacing_between = 4.0;
                let padding = 4.0;

                let tabs_data = [
                    (AppView::Home, "Home", icons::VIEW_HOME),
                    (AppView::Generate, "Generate", icons::VIEW_GENERATE),
                    (AppView::Review, "Review", icons::VIEW_REVIEW),
                    (AppView::Repos, "Repos", icons::VIEW_REPOS),
                    (AppView::Settings, "Settings", icons::VIEW_SETTINGS),
                ];

                // Measure each button's desired width
                // Since you always use the same font, we can estimate based on character count
                let mut button_widths = Vec::new();

                for (_, label, _) in &tabs_data {
                    // Approximate character width for 13.0 size font + padding
                    // Icon takes ~15px, each char ~7-8px for size 13
                    let estimated_width = 15.0 + (label.len() as f32 * 6.5) + 20.0; // padding
                    button_widths.push(estimated_width);
                }

                // Calculate total width needed
                let mut total_width = padding * 2.0; // Left and right padding
                for width in &button_widths {
                    total_width += width;
                }
                total_width += spacing_between * (tabs_data.len() - 1) as f32;

                // Create centered container rect based on measured width
                let container_height = 34.0;
                let center_rect = egui::Rect::from_center_size(
                    rect.center(),
                    egui::vec2(total_width, container_height),
                );

                // Draw background container
                let rounding = egui::CornerRadius::same(nav_rounding as u8);
                let stroke = egui::Stroke::new(1.0, theme.border);
                ui.painter()
                    .rect_filled(center_rect, rounding, theme.bg_secondary);
                ui.painter()
                    .rect_stroke(center_rect, rounding, stroke, egui::StrokeKind::Inside);

                // Render tabs inside the container
                let tab_rect = center_rect.shrink2(egui::vec2(padding, padding));
                ui.scope_builder(egui::UiBuilder::new().max_rect(tab_rect), |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(spacing_between, 0.0);

                        for (i, (view, label, icon)) in tabs_data.iter().enumerate() {
                            self.render_tab(ui, *view, label, icon, button_widths[i], nav_rounding);
                        }
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
        let is_active = self.state.ui.current_view == view;
        let theme = theme::current_theme();

        let (bg, stroke) = if is_active {
            (
                theme.bg_primary,
                egui::Stroke::new(1.0, theme.border.gamma_multiply(0.7)),
            )
        } else {
            (
                egui::Color32::TRANSPARENT,
                egui::Stroke::new(1.0, egui::Color32::TRANSPARENT),
            )
        };

        let mut text = typography::label(format!("{} {}", icon, label));

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
                .corner_radius(rounding - 2.0)
                .min_size(egui::vec2(width, 26.0));

            if ui.add(btn).clicked() {
                match view {
                    AppView::Home => self.switch_to_home(),
                    AppView::Generate => self.switch_to_generate(),
                    AppView::Review => self.switch_to_review(),
                    AppView::Repos => self.switch_to_repos(),
                    AppView::Settings => self.switch_to_settings(),
                }
            }
        });
    }
}
