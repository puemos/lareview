use crate::ui::app::AppView;
use crate::ui::app::LaReviewApp;
use crate::ui::icons;
use crate::ui::spacing;
use crate::ui::{theme, typography};
use eframe::egui;

impl LaReviewApp {
    pub(super) fn render_header(&mut self, ctx: &egui::Context) {
        let theme = theme::current_theme();
        let header_height = 52.0;

        egui::TopBottomPanel::top("header")
            .exact_height(header_height)
            .show(ctx, |ui| {
                let rect = ui.available_rect_before_wrap();

                // --- 1. LEFT: App Identity ---
                let left_rect =
                    egui::Rect::from_min_size(rect.min, egui::vec2(200.0, rect.height()));
                ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect), |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.add_space(spacing::SPACING_SM);
                        ui.label(
                            egui::RichText::new(icons::STATUS_IN_PROGRESS)
                                .size(22.0)
                                .color(theme.brand),
                        );
                        ui.add_space(2.0);
                        ui.label(
                            typography::bold("LaReview")
                                .size(14.0)
                                .color(theme.text_primary),
                        );
                    });
                });

                // --- 2. CENTER: Navigation Tabs ---
                let tabs_data = [
                    (AppView::Home, "Home", icons::VIEW_HOME),
                    (AppView::Generate, "Generate", icons::VIEW_GENERATE),
                    (AppView::Review, "Review", icons::VIEW_REVIEW),
                    (AppView::Repos, "Repos", icons::VIEW_REPOS),
                    (AppView::Settings, "Settings", icons::VIEW_SETTINGS),
                ];

                let font_id = typography::body_font(13.0);
                let icon_font_id = typography::body_font(14.0);
                let spacing_between = 4.0;
                let item_spacing = 6.0;
                let horizontal_padding = 10.0;

                let mut total_tabs_width = 0.0;
                let mut tab_widths = Vec::new();

                for (_, label, icon) in &tabs_data {
                    let l_g = ui.painter().layout_no_wrap(
                        label.to_string(),
                        font_id.clone(),
                        theme.text_primary,
                    );
                    let i_g = ui.painter().layout_no_wrap(
                        icon.to_string(),
                        icon_font_id.clone(),
                        theme.text_primary,
                    );
                    let w = i_g.size().x + item_spacing + l_g.size().x + (horizontal_padding * 2.0);
                    tab_widths.push(w);
                    total_tabs_width += w;
                }
                total_tabs_width += spacing_between * (tabs_data.len() - 1) as f32;

                let container_width = total_tabs_width + 12.0; // 6.0 inner margin on each side
                let container_height = 34.0;
                let center_rect = egui::Rect::from_center_size(
                    rect.center(),
                    egui::vec2(container_width, container_height),
                );

                ui.put(center_rect, |ui: &mut egui::Ui| {
                    egui::Frame::default()
                        .fill(theme.bg_secondary)
                        .stroke(egui::Stroke::new(1.0, theme.border))
                        .corner_radius(egui::CornerRadius::same(6))
                        .inner_margin(egui::Margin::symmetric(6, 4))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = spacing_between;

                                for (i, (view, label, icon)) in tabs_data.iter().enumerate() {
                                    self.render_tab(ui, *view, label, icon, tab_widths[i]);
                                }
                            });
                        })
                        .response
                });
            });
    }

    fn render_tab(
        &mut self,
        ui: &mut egui::Ui,
        view: AppView,
        label: &str,
        icon: &str,
        width: f32,
    ) {
        let is_active = self.state.ui.current_view == view;
        let is_generating = view == AppView::Generate && self.state.session.is_generating;
        let theme = theme::current_theme();

        let height = 26.0;
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());

        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));

        if response.clicked() {
            match view {
                AppView::Home => self.switch_to_home(),
                AppView::Generate => self.switch_to_generate(),
                AppView::Review => self.switch_to_review(),
                AppView::Repos => self.switch_to_repos(),
                AppView::Settings => self.switch_to_settings(),
            }
        }

        let mut text_color = if is_active {
            theme.brand
        } else if response.hovered() {
            theme.text_primary
        } else {
            theme.text_secondary
        };

        // Ensure full opacity
        let c = text_color.to_array();
        text_color = egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], 255);

        let painter = ui.painter();
        let font_id = typography::body_font(13.0);
        let icon_font_id = typography::body_font(14.0);
        let horizontal_padding = 10.0;
        let item_spacing = 6.0;

        // Draw Icon
        let icon_galley =
            painter.layout_no_wrap(icon.to_string(), icon_font_id.clone(), text_color);
        let icon_size = icon_galley.size();
        let icon_x = rect.min.x + horizontal_padding + (icon_size.x / 2.0);
        let icon_center = egui::pos2(icon_x, rect.center().y);

        if is_generating {
            let time = ui.input(|i| i.time);
            let angle = (time * 3.0) as f32;
            let rot = egui::emath::Rot2::from_angle(angle);
            let pos = icon_center - rot * (icon_galley.rect.center().to_vec2());

            painter.add(egui::Shape::Text(egui::epaint::TextShape {
                pos,
                galley: icon_galley,
                underline: egui::Stroke::NONE,
                override_text_color: Some(text_color),
                angle,
                fallback_color: text_color,
                opacity_factor: 1.0,
            }));
        } else {
            painter.text(
                icon_center,
                egui::Align2::CENTER_CENTER,
                icon,
                icon_font_id,
                text_color,
            );
        }

        // Draw Label
        let label_x = rect.min.x + horizontal_padding + icon_size.x + item_spacing;
        let label_pos = egui::pos2(label_x, rect.center().y);

        painter.text(
            label_pos,
            egui::Align2::LEFT_CENTER,
            label,
            font_id,
            text_color,
        );
    }
}
