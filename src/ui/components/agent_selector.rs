use eframe::egui;
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::ui::app::SelectedAgent;
use catppuccin_egui::MOCHA;

static LOGO_BYTES_CACHE: Lazy<Mutex<HashMap<String, Arc<[u8]>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn load_logo_bytes(path: &str) -> Option<Arc<[u8]>> {
    if let Ok(mut cache) = LOGO_BYTES_CACHE.lock() {
        if let Some(bytes) = cache.get(path) {
            return Some(bytes.clone());
        }

        let bytes: Arc<[u8]> = std::fs::read(path).ok()?.into();
        cache.insert(path.to_owned(), bytes.clone());
        Some(bytes)
    } else {
        std::fs::read(path).ok().map(Into::into)
    }
}

pub fn agent_selector(ui: &mut egui::Ui, selected_agent: &mut SelectedAgent) {
    let candidates = crate::infra::acp::list_agent_candidates();
    let selected_candidate = candidates
        .iter()
        .find(|c| c.id == selected_agent.to_string());

    let selected_label = selected_candidate
        .map(|c| c.label.clone())
        .unwrap_or_else(|| selected_agent.to_string());

    let selected_logo_path = selected_candidate.and_then(|c| c.logo.clone());

    ui.push_id("agent_selector_combo", |ui| {
        let id = ui.make_persistent_id("agent_selector_popup");
        let is_open = egui::Popup::is_id_open(ui.ctx(), id);

        // 1. Draw the "ComboBox" button manually
        let button_height = 28.0;
        let width = 200.0;

        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(width, button_height), egui::Sense::click());

        if response.clicked() {
            egui::Popup::toggle_id(ui.ctx(), id);
        }

        // Draw button background / border
        let visuals = ui.style().visuals.clone();
        let bg_fill = if is_open {
            visuals.widgets.open.bg_fill
        } else if response.hovered() {
            visuals.widgets.hovered.bg_fill
        } else {
            visuals.widgets.inactive.bg_fill
        };

        // Stroke
        let stroke = if is_open {
            visuals.widgets.open.fg_stroke
        } else if response.hovered() {
            visuals.widgets.hovered.fg_stroke
        } else {
            visuals.widgets.inactive.fg_stroke
        };

        ui.painter().rect(
            rect,
            visuals.widgets.inactive.corner_radius,
            bg_fill,
            stroke,
            egui::StrokeKind::Middle,
        );

        // Draw Content (Logo + Text + Chevron)
        // Use less vertical shrinking to allow correct centering
        let content_rect = rect.shrink2(egui::vec2(6.0, 0.0));
        let ui_builder = egui::UiBuilder::new().max_rect(content_rect);
        ui.scope_builder(ui_builder, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(2.0);
                // Logo
                if let Some(logo_path) = &selected_logo_path
                    && let Some(bytes) = load_logo_bytes(logo_path)
                {
                    let uri = format!("bytes://{}", logo_path);
                    let image = egui::Image::from_bytes(uri, bytes)
                        .fit_to_exact_size(egui::vec2(16.0, 16.0))
                        .corner_radius(2.0);
                    ui.add(image);
                    ui.add_space(4.0);
                }

                // Text
                ui.add(
                    egui::Label::new(egui::RichText::new(selected_label).color(MOCHA.text))
                        .selectable(false),
                );

                // Spacer
                ui.allocate_ui_with_layout(
                    ui.available_size(),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.add_space(2.0);
                        ui.add(egui::Label::new("⏷").selectable(false));
                    },
                );
            });
        });

        // 2. Draw Popup using the new Popup::show API
        egui::Popup::new(id, ui.ctx().clone(), rect, ui.layer_id())
            .open_memory(None) // Don't change state here, we handle it above with toggle_id
            .show(|ui| {
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        ui.set_width(width);

                        for agent in &candidates {
                            let is_selected = selected_agent.to_string() == agent.id;
                            let is_available = agent.available;

                            // Selection Logic - now with proper hover detection
                            let item_height = 28.0;

                            let (item_rect, item_response) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), item_height),
                                egui::Sense::click(),
                            );

                            if is_available {
                                item_response
                                    .clone()
                                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                            }

                            if item_response.clicked() && is_available {
                                *selected_agent = SelectedAgent::from_str(&agent.id);
                                egui::Popup::close_id(ui.ctx(), id);
                            }

                            // Hover/Select styling
                            let mut item_bg = egui::Color32::TRANSPARENT;
                            if is_selected {
                                item_bg = MOCHA.surface0;
                            } else if item_response.hovered() && is_available {
                                item_bg = MOCHA.surface0.gamma_multiply(0.5);
                            }

                            ui.painter().rect_filled(item_rect, 2.0, item_bg);

                            // Content - with better padding
                            let content_rect = item_rect.shrink2(egui::vec2(8.0, 4.0));
                            let item_ui = egui::UiBuilder::new().max_rect(content_rect);
                            ui.scope_builder(item_ui, |ui| {
                                ui.horizontal_centered(|ui| {
                                    // Logo
                                    if let Some(logo_path) = &agent.logo
                                        && let Some(bytes) = load_logo_bytes(logo_path)
                                    {
                                        let uri = format!("bytes://{}", logo_path);
                                        let image = egui::Image::from_bytes(uri, bytes)
                                            .fit_to_exact_size(egui::vec2(14.0, 14.0))
                                            .corner_radius(2.0);

                                        if !is_available {
                                            ui.add(
                                                image.tint(egui::Color32::from_white_alpha(100)),
                                            );
                                        } else {
                                            ui.add(image);
                                        }
                                        ui.add_space(6.0);
                                    }

                                    let text_color = if !is_available {
                                        ui.visuals().weak_text_color()
                                    } else if is_selected {
                                        MOCHA.green
                                    } else {
                                        MOCHA.text
                                    };

                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&agent.label).color(text_color),
                                        )
                                        .selectable(false),
                                    );
                                });
                            });

                            if !is_available && item_response.hovered() {
                                item_response.on_hover_text(
                                    "Agent not available. Please install it or add it to your PATH.",
                                );
                            }
                        }
                    });
            });

        // The Popup::show API handles keeping the popup open and closing on outside clicks automatically
    });
}
