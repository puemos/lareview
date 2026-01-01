use crate::ui::app::AppView;
use crate::ui::app::LaReviewApp;
use crate::ui::icons;
use crate::ui::spacing;
use crate::ui::{theme, typography};
use eframe::egui;
use once_cell::sync::Lazy;
use std::sync::Arc;

#[derive(Clone)]
struct HeaderLogo {
    uri: &'static str,
    bytes: Arc<[u8]>,
}

fn header_logo() -> Option<HeaderLogo> {
    static LOGO: Lazy<Option<HeaderLogo>> = Lazy::new(|| {
        crate::assets::get_content("assets/logo/512-light.svg").map(|bytes| HeaderLogo {
            uri: "bytes://header_logo.svg",
            bytes: bytes.into(),
        })
    });
    LOGO.clone()
}

impl LaReviewApp {
    pub(super) fn render_header(&mut self, ctx: &egui::Context) {
        let theme = theme::current_theme();
        let header_height = 36.0;

        egui::TopBottomPanel::top("header")
            .frame(egui::Frame::default().fill(theme.bg_surface))
            .exact_height(header_height)
            .show(ctx, |ui| {
                let rect = ui.available_rect_before_wrap();

                // --- 1. LEFT: Empty space for balance ---
                let left_rect =
                    egui::Rect::from_min_size(rect.min, egui::vec2(200.0, rect.height()));
                ui.advance_cursor_after_rect(left_rect);

                // --- 2. CENTER: Navigation Tabs ---
                let tabs_data = [
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

                // Calculate the total width of the navigation container, accounting for
                // the inner margin (6.0px) on each side.
                let container_width = total_tabs_width + 12.0;
                let container_height = 34.0;
                let center_rect = egui::Rect::from_center_size(
                    rect.center(),
                    egui::vec2(container_width, container_height),
                );

                ui.put(center_rect, |ui: &mut egui::Ui| {
                    egui::Frame::default()
                        .stroke(egui::Stroke::new(0.0, theme.border))
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

                // --- 3. RIGHT: App Identity ---
                let right_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.max.x - 200.0, rect.min.y),
                    egui::vec2(200.0, rect.height()),
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(right_rect), |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(spacing::SPACING_SM);
                        if let Some(logo) = header_logo() {
                            let image = egui::Image::from_bytes(logo.uri, logo.bytes)
                                .fit_to_exact_size(egui::vec2(16.0, 16.0))
                                .corner_radius(4.0);
                            ui.add(image);
                        }
                    });
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
