use catppuccin_egui::MOCHA;
use eframe::egui;
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

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

/// Selection chips component for any enum type
pub fn selection_chips<T>(
    ui: &mut egui::Ui,
    current_item: &mut T,
    items: &[T],
    labels: &[&str],
    logos: &[Option<String>],
    label_prefix: &str,
) where
    T: PartialEq + Clone,
{
    ui.horizontal_wrapped(|ui| {
        if !label_prefix.is_empty() {
            ui.label(label_prefix);
        }

        for (i, item) in items.iter().enumerate() {
            if i < labels.len() {
                let selected = current_item == item;
                let label = labels[i];
                let logo_path = logos.get(i).and_then(|l| l.as_ref());

                let frame = egui::Frame::new()
                    .fill(if selected { MOCHA.surface1 } else { MOCHA.base })
                    .stroke(egui::Stroke::new(1.0, MOCHA.surface2))
                    .inner_margin(egui::vec2(8.0, 4.0))
                    .corner_radius(egui::CornerRadius::same(20));

                let response = frame
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if let Some(path) = logo_path
                                && let Some(bytes) = load_logo_bytes(path)
                            {
                                let uri = format!("bytes://{path}");
                                ui.add(
                                    egui::Image::from_bytes(uri, bytes)
                                        .fit_to_exact_size(egui::vec2(16.0, 16.0)),
                                );
                            }
                            let text = egui::RichText::new(label)
                                .color(egui::Color32::from_rgb(255, 255, 255));
                            ui.label(text);
                        });
                    })
                    .response
                    .on_hover_cursor(egui::CursorIcon::PointingHand);

                if response.interact(egui::Sense::click()).clicked() {
                    *current_item = item.clone();
                }
            }
        }
    });
}
