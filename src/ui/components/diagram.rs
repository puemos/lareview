use egui::{Id, Image, Rect, Ui, load::Bytes};
use regex::Regex;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use twox_hash::XxHash64;

#[derive(Clone)]
enum DiagramState {
    Loading,
    Ready(String),
    Error(String),
}

/// Internal state stored in egui memory for diagram management
#[derive(Clone)]
struct DiagramMemory {
    cache: HashMap<String, DiagramState>,
    scene_rect: Rect,
    is_expanded: bool,
}

impl Default for DiagramMemory {
    fn default() -> Self {
        Self {
            cache: HashMap::new(),
            scene_rect: Rect::NOTHING,
            is_expanded: false,
        }
    }
}

// Global storage for receivers (can't be stored in egui memory due to Clone/Sync requirements)
lazy_static::lazy_static! {
    static ref DIAGRAM_RECEIVERS: Arc<Mutex<HashMap<String, Receiver<DiagramState>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

/// Renders a D2 diagram using egui Scene for zooming.
/// Manages all state internally using egui's memory system.
/// Returns true if the user clicks the "Go to Settings" link.
///
/// This is fully self-contained - just pass the diagram code and dark mode flag.
pub fn diagram_view(ui: &mut Ui, diagram: &Option<String>, is_dark_mode: bool) -> bool {
    let mut go_to_settings = false;

    // Generate stable ID for memory storage
    let memory_id = Id::new("d2_diagram_memory");

    // Load memory from egui
    let mut memory = ui.ctx().memory_mut(|mem| {
        mem.data
            .get_temp::<DiagramMemory>(memory_id)
            .unwrap_or_default()
    });

    let Some(d2_code) = diagram else {
        ui.centered_and_justified(|ui| {
            ui.label(egui::RichText::new("No diagram code provided").weak());
        });
        return false;
    };

    let trimmed_code = d2_code.trim();
    if trimmed_code.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(egui::RichText::new("Enter D2 code to render a diagram").weak());
        });
        return false;
    }

    // Cache key is code + theme
    let cache_key = format!("{}_{}", trimmed_code, is_dark_mode);

    // Check if we need to start generation
    if !memory.cache.contains_key(&cache_key) {
        memory
            .cache
            .insert(cache_key.clone(), DiagramState::Loading);

        // Create channel for receiving result
        let (tx, rx) = channel();

        // Store receiver in global storage
        if let Ok(mut receivers) = DIAGRAM_RECEIVERS.lock() {
            receivers.insert(cache_key.clone(), rx);
        }

        // Spawn background task to generate SVG
        let trimmed_code = trimmed_code.to_string();
        let ctx = ui.ctx().clone();

        std::thread::spawn(move || {
            let result = crate::ui::diagram::d2::d2_to_svg(&trimmed_code, is_dark_mode);

            let state = match result {
                Ok(svg) => DiagramState::Ready(svg),
                Err(e) => DiagramState::Error(e),
            };

            // Send result through channel
            let _ = tx.send(state);

            // Request repaint
            ctx.request_repaint();
        });
    }

    // Check if generation completed in background
    if matches!(memory.cache.get(&cache_key), Some(DiagramState::Loading))
        && let Ok(mut receivers) = DIAGRAM_RECEIVERS.lock()
        && let Some(rx) = receivers.get(&cache_key)
    {
        // Try to receive result without blocking
        if let Ok(completed_state) = rx.try_recv() {
            memory.cache.insert(cache_key.clone(), completed_state);
            receivers.remove(&cache_key);
        }
    }

    let state = memory.cache.get(&cache_key).unwrap().clone();

    // If expanded, show in full-screen overlay
    if memory.is_expanded {
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
                        .button(format!("{} Close", egui_phosphor::regular::X))
                        .clicked()
                    {
                        memory.is_expanded = false;
                    }
                    ui.label("Use mouse wheel to zoom, drag to pan");
                });

                ui.separator();

                render_diagram_content(
                    ui,
                    &state,
                    &cache_key,
                    trimmed_code,
                    &mut memory.scene_rect,
                    &mut go_to_settings,
                );
            });
    } else {
        // Inline compact view with expand button
        let desired_size = ui.available_size();

        egui::Frame::default()
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .show(ui, |ui| {
                ui.set_max_size(desired_size);

                // Show expand button
                ui.horizontal(|ui| {
                    if ui
                        .button(format!("{} Expand", egui_phosphor::regular::ARROWS_OUT))
                        .clicked()
                    {
                        memory.is_expanded = true;
                    }
                    ui.label(
                        egui::RichText::new("Click expand to zoom and pan")
                            .weak()
                            .italics(),
                    );
                });

                ui.add_space(8.0);

                // Render static preview (no interaction)
                render_diagram_preview(
                    ui,
                    &state,
                    &cache_key,
                    trimmed_code,
                    desired_size,
                    &mut go_to_settings,
                );
            });
    }

    // Save memory back to egui
    ui.ctx().memory_mut(|mem| {
        mem.data.insert_temp(memory_id, memory);
    });

    go_to_settings
}

/// Render the interactive diagram content (used in expanded view)
fn render_diagram_content(
    ui: &mut Ui,
    state: &DiagramState,
    cache_key: &str,
    trimmed_code: &str,
    scene_rect: &mut Rect,
    go_to_settings: &mut bool,
) {
    match state {
        DiagramState::Loading => {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.spinner();
                    ui.add_space(8.0);
                    ui.label("Generating diagram...");
                });
            });
            ui.ctx().request_repaint();
        }
        DiagramState::Ready(svg_code) => {
            let original_image_size = extract_svg_size(svg_code);

            // Initialize scene_rect if it's invalid
            if !scene_rect.is_positive() {
                *scene_rect = Rect::from_min_size(egui::pos2(0.0, 0.0), original_image_size);
            }

            let image_id = generate_image_id(cache_key);
            let image_bytes = Bytes::from(svg_code.as_bytes().to_vec());

            // Use Scene for zoomable/pannable view
            egui::Scene::new()
                .zoom_range(0.1..=5.0)
                .show(ui, scene_rect, |ui| {
                    let image_widget = Image::from_bytes(image_id, image_bytes)
                        .fit_to_exact_size(original_image_size);

                    ui.add(image_widget);
                });
        }
        DiagramState::Error(e) => {
            render_error(ui, e, trimmed_code, go_to_settings);
        }
    }
}

/// Render the static preview (used in inline view)
fn render_diagram_preview(
    ui: &mut Ui,
    state: &DiagramState,
    cache_key: &str,
    trimmed_code: &str,
    desired_size: egui::Vec2,
    go_to_settings: &mut bool,
) {
    match state {
        DiagramState::Loading => {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.spinner();
                    ui.add_space(8.0);
                    ui.label("Generating diagram...");
                });
            });
            ui.ctx().request_repaint();
        }
        DiagramState::Ready(svg_code) => {
            let original_image_size = extract_svg_size(svg_code);

            // Scale to fit available space while maintaining aspect ratio
            let scale = (desired_size.x / original_image_size.x)
                .min(desired_size.y / original_image_size.y)
                .min(1.0);
            let preview_size = original_image_size * scale;

            let image_id = generate_image_id(cache_key);
            let image_bytes = Bytes::from(svg_code.as_bytes().to_vec());

            ui.vertical_centered(|ui| {
                let image_widget =
                    Image::from_bytes(image_id, image_bytes).fit_to_exact_size(preview_size);

                ui.add(image_widget);
            });
        }
        DiagramState::Error(e) => {
            render_error(ui, e, trimmed_code, go_to_settings);
        }
    }
}

/// Extract SVG viewBox size
fn extract_svg_size(svg_code: &str) -> egui::Vec2 {
    lazy_static::lazy_static! {
        static ref VIEWBOX_REGEX: Regex = Regex::new(
            r#"viewBox=["']\s*(\S+)\s+(\S+)\s+(\S+)\s+(\S+)\s*["']"#
        ).unwrap();
    }

    let mut size = egui::vec2(500.0, 500.0);

    if let Some(caps) = VIEWBOX_REGEX.captures(svg_code)
        && let (Some(width_str), Some(height_str)) = (caps.get(3), caps.get(4))
        && let (Ok(width), Ok(height)) = (
            width_str.as_str().parse::<f32>(),
            height_str.as_str().parse::<f32>(),
        )
        && width > 0.0
        && height > 0.0
    {
        size = egui::vec2(width, height);
    }

    size
}

/// Generate stable image ID
fn generate_image_id(cache_key: &str) -> String {
    let mut hasher = XxHash64::with_seed(0);
    cache_key.hash(&mut hasher);
    let hash_value = hasher.finish();
    format!("bytes://d2_diagram_{:x}.svg", hash_value)
}

/// Render error state
fn render_error(ui: &mut Ui, error: &str, trimmed_code: &str, go_to_settings: &mut bool) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.label("❌ Failed to render diagram:");
            ui.add_space(8.0);
            ui.label(egui::RichText::new(error).color(ui.visuals().warn_fg_color));

            if error.contains("D2 executable not found") {
                ui.add_space(8.0);
                if ui.link("Go to Settings to install D2").clicked() {
                    *go_to_settings = true;
                }
            }

            ui.add_space(16.0);
            ui.label("D2 Code:");
            ui.code(trimmed_code);
        });
    });
}
