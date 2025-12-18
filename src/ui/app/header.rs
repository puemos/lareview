use eframe::egui;
use egui_phosphor::regular::COFFEE;
use egui_phosphor::regular::ONIGIRI;

use super::LaReviewApp;
use super::state::AppView;
use crate::ui::spacing;
use crate::ui::theme;

impl LaReviewApp {
    pub(super) fn render_header(&mut self, ctx: &egui::Context) {
        let theme = theme::current_theme();

        egui::TopBottomPanel::top("header")
            .frame(egui::Frame::NONE.fill(theme.bg_secondary))
            .show(ctx, |ui| {
                let rect = ui.available_rect_before_wrap();

                // TUI-style bottom border for the header
                ui.painter().line_segment(
                    [
                        egui::pos2(rect.min.x, rect.max.y - 1.0),
                        egui::pos2(rect.max.x, rect.max.y - 1.0),
                    ],
                    egui::Stroke::new(1.0, theme.border),
                );

                ui.add_space(spacing::SPACING_SM);

                // Create a response for the whole header for dragging
                let header_response = ui.interact(
                    rect,
                    ui.id().with("header_drag"),
                    egui::Sense::click_and_drag(),
                );

                // If the header is dragged, start window drag
                if header_response.dragged() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                // Get the full header rect for absolute positioning
                let header_rect = ui.available_rect_before_wrap();
                let header_width = header_rect.width();

                ui.horizontal(|ui| {
                    // Left section: Window Controls and Logo
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
                            .size(16.0),
                    );
                    // Calculate center position for navigation
                    let nav_width = 380.0; // wider to fit Settings too
                    let center_x = header_rect.min.x + (header_width / 2.0) - (nav_width / 2.0);
                    let current_x = ui.cursor().min.x;
                    let space_to_center = (center_x - current_x).max(0.0);

                    ui.add_space(space_to_center);

                    // Center section: Navigation buttons (TUI-style)
                    let is_generate = self.state.current_view == AppView::Generate;
                    let generate_text = if is_generate {
                        format!("[{} GENERATE]", ONIGIRI)
                    } else {
                        format!(" {} GENERATE ", ONIGIRI)
                    };

                    let generate_response = ui.add(
                        egui::Button::new(egui::RichText::new(generate_text).color(
                            if is_generate {
                                theme.brand
                            } else {
                                theme.text_disabled
                            },
                        ))
                        .frame(false)
                        .corner_radius(egui::CornerRadius::ZERO),
                    );
                    if generate_response.clicked() {
                        self.switch_to_generate();
                    }

                    ui.add_space(spacing::SPACING_MD);

                    let is_review = self.state.current_view == AppView::Review;
                    let task_count = self.state.all_tasks.len();
                    let review_label = if task_count > 0 {
                        format!("REVIEW ({})", task_count)
                    } else {
                        "REVIEW".to_string()
                    };

                    let review_text = if is_review {
                        format!("[{} {}]", COFFEE, review_label)
                    } else {
                        format!(" {} {} ", COFFEE, review_label)
                    };

                    let review_response = ui.add(
                        egui::Button::new(egui::RichText::new(review_text).color(if is_review {
                            theme.brand
                        } else {
                            theme.text_disabled
                        }))
                        .frame(false)
                        .corner_radius(egui::CornerRadius::ZERO),
                    );
                    let review_response =
                        review_response.on_hover_cursor(egui::CursorIcon::PointingHand);
                    if review_response.clicked() {
                        self.switch_to_review();
                    }

                    ui.add_space(spacing::SPACING_MD);

                    // Settings button (Now in center)
                    let is_settings = self.state.current_view == AppView::Settings;
                    let settings_text = if is_settings {
                        "[SETTINGS]"
                    } else {
                        " SETTINGS "
                    };

                    let settings_response = ui.add(
                        egui::Button::new(egui::RichText::new(settings_text).color(
                            if is_settings {
                                theme.brand
                            } else {
                                theme.text_disabled
                            },
                        ))
                        .frame(false)
                        .corner_radius(egui::CornerRadius::ZERO),
                    );
                    let settings_response =
                        settings_response.on_hover_cursor(egui::CursorIcon::PointingHand);
                    if settings_response.clicked() {
                        self.switch_to_settings();
                    }

                    // Right section: Just spacing now
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(spacing::SPACING_SM);
                    });
                });
                ui.add_space(spacing::SPACING_SM);
            });
    }
}
