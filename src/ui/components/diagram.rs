use egui::{
    Id, Image, Rect, TextureOptions, Ui,
    load::{Bytes, SizeHint, SizedTexture, TexturePoll},
};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use twox_hash::XxHash64;

type DiagramKey = u64;

const SVG_RASTER_SIZE: u32 = 2048;

#[derive(Clone)]
struct DiagramSvg {
    image_id: String,
    bytes: Bytes,
}

#[derive(Clone, Copy)]
struct CachedTexture {
    id: egui::TextureId,
    size: egui::Vec2,
}

#[derive(Clone)]
enum DiagramState {
    Loading,
    SvgReady(DiagramSvg),
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

// Global storage for receivers (can't be stored in egui memory due to Clone/Sync requirements)
lazy_static::lazy_static! {
    static ref DIAGRAM_RECEIVERS: Arc<Mutex<HashMap<DiagramKey, Receiver<DiagramState>>>> =
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

    let diagram_key = diagram_key(trimmed_code, is_dark_mode);

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

            // Spawn background task to generate SVG
            let trimmed_code = trimmed_code.to_string();
            let ctx = ui.ctx().clone();
            let diagram_key_for_thread = diagram_key;

            std::thread::spawn(move || {
                let result = crate::infra::d2::d2_to_svg(&trimmed_code, is_dark_mode);

                let state = match result {
                    Ok(svg) => DiagramState::SvgReady(DiagramSvg {
                        image_id: image_id_for_key(diagram_key_for_thread),
                        bytes: Bytes::from(svg.into_bytes()),
                    }),
                    Err(e) => DiagramState::Error(e),
                };

                // Send result through channel
                let _ = tx.send(state);

                // Request repaint
                ctx.request_repaint();
            });
        }

        // Check if generation completed in background
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

    if let DiagramState::SvgReady(svg) = &state {
        ui.ctx()
            .include_bytes(svg.image_id.clone(), svg.bytes.clone());
        let load = ui.ctx().try_load_texture(
            svg.image_id.as_str(),
            TextureOptions::LINEAR,
            SizeHint::Size {
                width: SVG_RASTER_SIZE,
                height: SVG_RASTER_SIZE,
                maintain_aspect_ratio: true,
            },
        );

        match load {
            Ok(TexturePoll::Ready { texture }) => {
                let cached_texture = CachedTexture {
                    id: texture.id,
                    size: texture.size,
                };
                ui.ctx().memory_mut(|mem| {
                    let memory = mem.data.get_temp_mut_or_default::<DiagramMemory>(memory_id);
                    memory
                        .cache
                        .insert(diagram_key, DiagramState::TextureReady(cached_texture));
                });
                state = DiagramState::TextureReady(cached_texture);
            }
            Ok(TexturePoll::Pending { .. }) => {
                ui.ctx().request_repaint();
            }
            Err(e) => {
                ui.ctx().memory_mut(|mem| {
                    let memory = mem.data.get_temp_mut_or_default::<DiagramMemory>(memory_id);
                    memory
                        .cache
                        .insert(diagram_key, DiagramState::Error(e.to_string()));
                });
                state = DiagramState::Error(e.to_string());
            }
        }
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
                        .button(format!("{} Close", egui_phosphor::regular::X))
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
                    trimmed_code,
                    &mut scene_rect,
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

                let diagram_response = render_diagram(
                    ui,
                    &state,
                    trimmed_code,
                    &mut scene_rect,
                    &mut go_to_settings,
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
) -> Option<egui::Response> {
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
            None
        }
        DiagramState::SvgReady(_) => {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.spinner();
                    ui.add_space(8.0);
                    ui.label("Preparing diagram...");
                });
            });
            ui.ctx().request_repaint();
            None
        }
        DiagramState::TextureReady(texture) => {
            let input_used_for_pan_or_zoom = ui
                .ctx()
                .input(|i| i.smooth_scroll_delta != egui::Vec2::ZERO || i.zoom_delta() != 1.0);

            let inner = egui::Scene::new()
                .zoom_range(0.1..=8.0)
                .max_inner_size(egui::Vec2::splat(10_000.0))
                .show(ui, scene_rect, |ui| {
                    ui.add(
                        Image::from_texture(SizedTexture::new(texture.id, texture.size))
                            .fit_to_exact_size(texture.size),
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
