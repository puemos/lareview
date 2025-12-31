use crate::infra::diagram::{D2Renderer, Renderer, parse_json};
use crate::ui::theme::current_theme;
use crate::ui::{icons, typography};
use egui::{Id, Image, Rect, TextureOptions, Ui, load::SizedTexture};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use twox_hash::XxHash64;

type DiagramKey = u64;

const SVG_RASTER_SIZE: u32 = 2048;

// DiagramSvg removed - now using PixelsReady(egui::ColorImage)

#[derive(Clone)]
struct CachedTexture {
    handle: egui::TextureHandle,
}

#[derive(Clone)]
enum DiagramState {
    Loading,
    PixelsReady(Box<egui::ColorImage>),
    TextureReady(CachedTexture),
    Error(String),
}

/// Internal state stored in egui memory for diagram management
#[derive(Clone)]
struct DiagramMemory {
    cache: HashMap<DiagramKey, DiagramState>,
    scene_rect: Rect,
    last_key: Option<DiagramKey>,
    is_expanded: bool,
}

impl Default for DiagramMemory {
    fn default() -> Self {
        Self {
            cache: HashMap::new(),
            scene_rect: Rect::NOTHING,
            last_key: None,
            is_expanded: false,
        }
    }
}

// Global storage for receivers and fonts
lazy_static::lazy_static! {
    static ref DIAGRAM_RECEIVERS: Arc<Mutex<HashMap<DiagramKey, Receiver<DiagramState>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

/// Renders a D2 diagram using egui Scene for zooming.
/// Manages all state internally using egui's memory system.
/// Returns true if the user clicks the "Go to Settings" link.
///
/// This is fully self-contained - just pass the diagram code and dark mode flag.
pub fn diagram_view(ui: &mut Ui, diagram: &Option<Arc<str>>, is_dark_mode: bool) -> bool {
    let mut go_to_settings = false;

    // Generate stable ID for memory storage
    let memory_id = Id::new("d2_diagram_memory");

    let Some(diagram_code) = diagram else {
        ui.centered_and_justified(|ui| {
            ui.label(typography::weak("No diagram code provided"));
        });
        return false;
    };

    let trimmed_code = normalize_diagram_code(diagram_code);
    if trimmed_code.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(typography::weak("Enter diagram JSON to render"));
        });
        return false;
    }

    // Parse JSON -> Diagram -> D2
    let diagram_model = match parse_json(&trimmed_code) {
        Ok(d) => d,
        Err(e) => {
            ui.centered_and_justified(|ui| {
                ui.label(typography::bold(format!("Invalid diagram JSON: {e}")));
            });
            return false;
        }
    };

    let d2_code = match D2Renderer.render(&diagram_model) {
        Ok(code) => code,
        Err(e) => {
            ui.centered_and_justified(|ui| {
                ui.label(typography::bold(format!("D2 render error: {e}")));
            });
            return false;
        }
    };

    let diagram_key = diagram_key(&d2_code, is_dark_mode);

    let (mut state, is_expanded, mut scene_rect) = ui.ctx().memory_mut(|mem| {
        let memory = mem.data.get_temp_mut_or_default::<DiagramMemory>(memory_id);

        if memory.last_key != Some(diagram_key) {
            memory.scene_rect = Rect::NOTHING;
            memory.last_key = Some(diagram_key);
        }

        // Check if we need to start generation
        if let std::collections::hash_map::Entry::Vacant(entry) = memory.cache.entry(diagram_key) {
            entry.insert(DiagramState::Loading);

            // Create channel for receiving result
            let (tx, rx) = channel();

            // Store receiver in global storage
            if let Ok(mut receivers) = DIAGRAM_RECEIVERS.lock() {
                receivers.insert(diagram_key, rx);
            }

            // Spawn background task to generate SVG and rasterize it
            let d2_code = d2_code.clone();
            let ctx = ui.ctx().clone();

            std::thread::spawn(move || {
                let result = crate::infra::diagram::d2::d2_to_svg(&d2_code, is_dark_mode);

                let state = match result {
                    Ok(svg_str) => {
                        // Background Rasterization using resvg
                        let mut opts = usvg::Options::default();
                        let mut fontdb = fontdb::Database::new();
                        fontdb.load_system_fonts();
                        opts.fontdb = Arc::new(fontdb);

                        let rtree = usvg::Tree::from_str(&svg_str, &opts);

                        match rtree {
                            Ok(rtree) => {
                                let size = rtree.size();

                                // Calculate scaling to fit SVG_RASTER_SIZE while maintaining aspect ratio
                                let scale = (SVG_RASTER_SIZE as f32 / size.width())
                                    .min(SVG_RASTER_SIZE as f32 / size.height());

                                let mut pixmap = tiny_skia::Pixmap::new(
                                    (size.width() * scale).ceil() as u32,
                                    (size.height() * scale).ceil() as u32,
                                )
                                .unwrap();

                                resvg::render(
                                    &rtree,
                                    tiny_skia::Transform::from_scale(scale, scale),
                                    &mut pixmap.as_mut(),
                                );

                                let pixels = pixmap.data();
                                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                    [pixmap.width() as usize, pixmap.height() as usize],
                                    pixels,
                                );

                                DiagramState::PixelsReady(Box::new(color_image))
                            }
                            Err(e) => DiagramState::Error(format!("SVG parsing error: {}", e)),
                        }
                    }
                    Err(e) => DiagramState::Error(e),
                };

                // Send result through channel
                let _ = tx.send(state);

                // Request repaint
                ctx.request_repaint();
            });
        }

        if matches!(memory.cache.get(&diagram_key), Some(DiagramState::Loading))
            && let Ok(mut receivers) = DIAGRAM_RECEIVERS.lock()
            && let Some(rx) = receivers.get(&diagram_key)
        {
            // Try to receive result without blocking
            if let Ok(completed_state) = rx.try_recv() {
                memory.cache.insert(diagram_key, completed_state);
                receivers.remove(&diagram_key);
            }
        }

        (
            memory
                .cache
                .get(&diagram_key)
                .cloned()
                .unwrap_or(DiagramState::Loading),
            memory.is_expanded,
            memory.scene_rect,
        )
    });

    if let DiagramState::PixelsReady(color_image) = state {
        let texture = ui.ctx().load_texture(
            image_id_for_key(diagram_key),
            *color_image,
            TextureOptions::LINEAR,
        );

        let cached_texture = CachedTexture { handle: texture };

        ui.ctx().memory_mut(|mem| {
            let memory = mem.data.get_temp_mut_or_default::<DiagramMemory>(memory_id);
            memory.cache.insert(
                diagram_key,
                DiagramState::TextureReady(cached_texture.clone()),
            );
        });

        state = DiagramState::TextureReady(cached_texture);
    }

    // If expanded, show in full-screen overlay
    let mut next_is_expanded = is_expanded;
    if is_expanded {
        let viewport_rect = ui
            .ctx()
            .input(|i| i.viewport().inner_rect)
            .unwrap_or_else(|| {
                let rect = ui.ctx().content_rect();
                egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), rect.size())
            });

        egui::Window::new("Diagram Viewer")
            .fixed_rect(viewport_rect)
            .collapsible(false)
            .resizable(false)
            .title_bar(true)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(format!("{} Close", icons::ACTION_CLOSE))
                        .clicked()
                    {
                        next_is_expanded = false;
                    }
                    ui.label("Scroll to zoom, drag to pan");
                });

                ui.separator();

                let _ = render_diagram(
                    ui,
                    &state,
                    &trimmed_code,
                    &mut scene_rect,
                    &mut go_to_settings,
                    true, // is_expanded
                );
            });
    } else {
        // Inline compact view with expand button
        let desired_size = ui.available_size();

        egui::Frame::default()
            .stroke(egui::Stroke::new(1.0, current_theme().border))
            .corner_radius(egui::CornerRadius::ZERO)
            .show(ui, |ui| {
                ui.set_max_size(desired_size);

                let diagram_response = render_diagram(
                    ui,
                    &state,
                    &trimmed_code,
                    &mut scene_rect,
                    &mut go_to_settings,
                    false, // is_expanded
                );
                if let Some(resp) = diagram_response
                    && resp.clicked()
                {
                    next_is_expanded = true;
                }
            });
    }

    if next_is_expanded != is_expanded {
        ui.ctx().memory_mut(|mem| {
            let memory = mem.data.get_temp_mut_or_default::<DiagramMemory>(memory_id);
            memory.is_expanded = next_is_expanded;
        });
    }

    ui.ctx().memory_mut(|mem| {
        let memory = mem.data.get_temp_mut_or_default::<DiagramMemory>(memory_id);
        memory.scene_rect = scene_rect;
    });

    go_to_settings
}

fn render_diagram(
    ui: &mut Ui,
    state: &DiagramState,
    trimmed_code: &str,
    scene_rect: &mut Rect,
    go_to_settings: &mut bool,
    is_expanded: bool,
) -> Option<egui::Response> {
    match state {
        DiagramState::Loading | DiagramState::PixelsReady(_) => {
            let theme = current_theme();
            let time = ui.input(|i| i.time);

            // Use available width but force a reasonable height for centering if it's too small
            let available = ui.available_size();
            let min_height = 200.0;
            let size = egui::vec2(available.x, available.y.max(min_height));
            let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());

            let painter = ui.painter_at(rect);
            let center = rect.center();

            crate::ui::animations::cyber::paint_cyber_loader(
                &painter,
                center,
                "Rendering Diagram...",
                time,
                theme.brand,
                theme.text_muted,
            );

            ui.ctx().request_repaint();
            None
        }
        DiagramState::TextureReady(texture) => {
            if is_expanded {
                let input_used_for_pan_or_zoom = ui
                    .ctx()
                    .input(|i| i.smooth_scroll_delta != egui::Vec2::ZERO || i.zoom_delta() != 1.0);

                let inner = egui::Scene::new()
                    .zoom_range(0.1..=8.0)
                    .max_inner_size(egui::Vec2::splat(10_000.0))
                    .show(ui, scene_rect, |ui| {
                        ui.add(
                            Image::from_texture(SizedTexture::new(
                                texture.handle.id(),
                                texture.handle.size_vec2(),
                            ))
                            .fit_to_exact_size(texture.handle.size_vec2()),
                        );
                    });

                // `Scene` uses scroll input for pan/zoom; clear it so any parent `ScrollArea` won't also scroll.
                if inner.response.contains_pointer() && input_used_for_pan_or_zoom {
                    ui.ctx().input_mut(|input| {
                        input.smooth_scroll_delta = egui::Vec2::ZERO;
                        input.raw_scroll_delta = egui::Vec2::ZERO;
                    });
                }

                Some(inner.response)
            } else {
                // Compact view: static image, centered, clickable
                let available = ui.available_size();
                let min_height = 200.0;
                let size = egui::vec2(available.x, available.y.max(min_height));
                let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

                ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.add(
                            Image::from_texture(SizedTexture::new(
                                texture.handle.id(),
                                texture.handle.size_vec2(),
                            ))
                            .shrink_to_fit(),
                        )
                    })
                });
                Some(response.on_hover_cursor(egui::CursorIcon::PointingHand))
            }
        }
        DiagramState::Error(e) => {
            render_error(ui, e, trimmed_code, go_to_settings);
            None
        }
    }
}

fn diagram_key(code: &str, is_dark_mode: bool) -> DiagramKey {
    let mut hasher = XxHash64::with_seed(0);
    code.hash(&mut hasher);
    is_dark_mode.hash(&mut hasher);
    hasher.finish()
}

/// Generate stable image ID
fn image_id_for_key(diagram_key: DiagramKey) -> String {
    format!("bytes://d2_diagram_{:x}.svg", diagram_key)
}

/// Render error state
fn render_error(ui: &mut Ui, error: &str, trimmed_code: &str, go_to_settings: &mut bool) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.label(
                typography::body("Failed to load diagram.").color(current_theme().destructive),
            );
            ui.add_space(8.0);
            ui.label(typography::weak(error));

            if error.contains("D2 executable not found") {
                ui.add_space(8.0);
                if ui.button("Install D2").clicked() {
                    *go_to_settings = true;
                }
            }

            ui.add_space(16.0);
            ui.label("Diagram JSON:");
            ui.code(trimmed_code);
        });
    });
}

fn normalize_diagram_code(code: &str) -> String {
    code.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    #[test]
    fn test_diagram_view_no_code() {
        let mut harness = Harness::new(|ctx| {
            crate::ui::app::LaReviewApp::setup_fonts(ctx);
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.style_mut().override_font_id = Some(egui::FontId::proportional(12.0));
                diagram_view(ui, &None, true);
            });
        });
        harness.run_steps(5);
        harness.get_by_label("No diagram code provided");
    }

    #[test]
    fn test_diagram_view_empty_code() {
        let mut harness = Harness::new(|ctx| {
            crate::ui::app::LaReviewApp::setup_fonts(ctx);
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.style_mut().override_font_id = Some(egui::FontId::proportional(12.0));
                diagram_view(ui, &Some(Arc::from("  ")), true);
            });
        });
        harness.run_steps(5);
        harness.get_by_label("Enter diagram JSON to render");
    }

    #[test]
    fn test_diagram_view_error_state() {
        // Manually inject error state into memory
        let mut harness = Harness::new(|ctx| {
            crate::ui::app::LaReviewApp::setup_fonts(ctx);

            let code = "x -> y";
            let key = diagram_key(code, true);
            let memory_id = Id::new("d2_diagram_memory");
            ctx.memory_mut(|mem| {
                let memory = mem.data.get_temp_mut_or_default::<DiagramMemory>(memory_id);
                memory
                    .cache
                    .insert(key, DiagramState::Error("Invalid diagram JSON: Diagram parse error: JSON parse error: expected value at line 1 column 1".into()));
                memory.last_key = Some(key);
            });

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.style_mut().override_font_id = Some(egui::FontId::proportional(12.0));
                diagram_view(ui, &Some(Arc::from(code)), true);
            });
        });
        harness.run_steps(5);
        harness.get_by_label_contains("Invalid diagram JSON");
    }

    #[test]
    fn test_normalize_diagram_code() {
        assert_eq!(normalize_diagram_code("  {}  "), "{}");
        assert_eq!(
            normalize_diagram_code("{\"a\": 1}\\n{\"b\": 2}"),
            "{\"a\": 1}\\n{\"b\": 2}"
        );
    }

    #[test]
    fn test_diagram_key() {
        let k1 = diagram_key("a", true);
        let k2 = diagram_key("a", false);
        let k3 = diagram_key("b", true);
        assert_ne!(k1, k2);
        assert_ne!(k1, k3);
    }

    #[test]
    fn test_render_error() {
        use std::sync::{Arc, Mutex};
        let go_to_settings = Arc::new(Mutex::new(false));
        let go_to_settings_clone = go_to_settings.clone();
        let mut harness = Harness::new_ui(move |ui| {
            let mut guard = go_to_settings_clone.lock().unwrap();
            render_error(ui, "D2 executable not found", "x -> y", &mut guard);
        });
        harness.run();
        harness.get_by_label("Install D2").click();
        harness.run();
        assert!(*go_to_settings.lock().unwrap());
    }
}
