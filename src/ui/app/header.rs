use eframe::egui;
use egui_phosphor::regular::COFFEE;
use egui_phosphor::regular::ONIGIRI;

use super::LaReviewApp;
use super::state::AppView;
use crate::ui::spacing;
use crate::ui::theme;

impl LaReviewApp {
    pub(super) fn render_header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            let theme = theme::current_theme();
            let rect = ui.available_rect_before_wrap();
            ui.painter().with_clip_rect(rect).rect_filled(
                rect,
                egui::CornerRadius::ZERO,
                theme.bg_primary,
            );

            let line_spacing = 20.0; // Keep original value to avoid type issue
            let line_width = 0.5;
            let color = theme.bg_secondary.linear_multiply(0.25);

            let mut pos = rect.min.x - rect.max.y;
            while pos < rect.max.x {
                ui.painter().line_segment(
                    [
                        egui::Pos2::new(pos, rect.min.y),
                        egui::Pos2::new(pos + (rect.max.y - rect.min.y), rect.max.y),
                    ],
                    egui::Stroke::new(line_width, color),
                );
                pos += line_spacing;
            }

            let mut pos = rect.min.y;
            while pos < rect.max.y {
                ui.painter().line_segment(
                    [
                        egui::Pos2::new(rect.min.x, pos),
                        egui::Pos2::new(rect.min.x + (rect.max.y - pos), rect.max.y),
                    ],
                    egui::Stroke::new(line_width, color),
                );
                pos += line_spacing;
            }

            ui.add_space(spacing::SPACING_SM);

            // Get the full header rect for absolute positioning
            let header_rect = ui.available_rect_before_wrap();
            let header_width = header_rect.width();

            ui.horizontal(|ui| {
                // Left section: Logo
                ui.add_space(spacing::SPACING_MD);
                match ui.ctx().try_load_texture(
                    "app_logo",
                    egui::TextureOptions::LINEAR,
                    Default::default(),
                ) {
                    Ok(egui::load::TexturePoll::Ready { texture }) => {
                        ui.image(texture);
                    }
                    Ok(egui::load::TexturePoll::Pending { .. }) | Err(_) => {
                        ui.add(egui::Label::new(
                            egui::RichText::new(egui_phosphor::regular::CIRCLE_HALF)
                                .size(22.0)
                                .color(theme.brand),
                        ));
                    }
                }
                ui.heading(
                    egui::RichText::new("LaReview")
                        .strong()
                        .color(theme.text_primary)
                        .size(18.0),
                );

                // Calculate center position for navigation
                // We'll use the header width to position nav buttons in the absolute center
                let nav_width = 200.0; // Approximate width of GENERATE + REVIEW buttons
                let center_x = header_rect.min.x + (header_width / 2.0) - (nav_width / 2.0);
                let current_x = ui.cursor().min.x;
                let space_to_center = (center_x - current_x).max(0.0);

                ui.add_space(space_to_center);

                // Center section: Navigation buttons
                let generate_response = ui.add(
                    egui::Button::new(egui::RichText::new(format!("{} GENERATE", ONIGIRI)).color(
                        if self.state.current_view == AppView::Generate {
                            theme.brand
                        } else {
                            theme.text_disabled
                        },
                    ))
                    .frame(false)
                    .corner_radius(egui::CornerRadius::same(4)),
                );
                if generate_response.clicked() {
                    self.switch_to_generate();
                }

                ui.add_space(spacing::SPACING_LG);

                let review_response = ui.add(
                    egui::Button::new(egui::RichText::new(format!("{} REVIEW", COFFEE)).color(
                        if self.state.current_view == AppView::Review {
                            theme.brand
                        } else {
                            theme.text_disabled
                        },
                    ))
                    .frame(false)
                    .corner_radius(egui::CornerRadius::same(4)),
                );
                let review_response =
                    review_response.on_hover_cursor(egui::CursorIcon::PointingHand);
                if review_response.clicked() {
                    self.switch_to_review();
                }

                // Right section: Settings (right-aligned)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(spacing::SPACING_MD);
                    let settings_response = ui.add(
                        egui::Button::new(egui::RichText::new("SETTINGS").color(
                            if self.state.current_view == AppView::Settings {
                                theme.brand
                            } else {
                                theme.text_disabled
                            },
                        ))
                        .frame(false)
                        .corner_radius(egui::CornerRadius::same(4)),
                    );
                    let settings_response =
                        settings_response.on_hover_cursor(egui::CursorIcon::PointingHand);
                    if settings_response.clicked() {
                        self.switch_to_settings();
                    }
                });
            });
            ui.add_space(spacing::SPACING_SM);
        });
    }
}
