use eframe::egui;
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::ui::theme;

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
    // configure wrapping gap
    let spacing = egui::vec2(8.0, 8.0);
    ui.spacing_mut().item_spacing = spacing;

    ui.horizontal_wrapped(|wrap_ui| {
        if !label_prefix.is_empty() {
            wrap_ui.label(label_prefix);
        }

        for (i, item) in items.iter().enumerate() {
            if i >= labels.len() {
                continue;
            }

            let selected = current_item == item;
            let label = labels[i];
            let logo_path = logos.get(i).and_then(|l| l.as_ref());

            let theme = theme::current_theme();
            let frame = egui::Frame::new()
                .fill(if selected {
                    theme.bg_card
                } else {
                    theme.bg_primary
                })
                .stroke(egui::Stroke::new(1.0, theme.border))
                .inner_margin(egui::vec2(8.0, 4.0))
                .corner_radius(egui::CornerRadius::same(20));

            // each chip gets a small constrained child ui
            let available_width = wrap_ui.available_width();
            let response = frame
                .show(wrap_ui, |inner_ui| {
                    // allow inner to shrink and wrap
                    inner_ui.set_min_width(0.0);
                    inner_ui.set_max_width(available_width);

                    inner_ui.horizontal(|h_ui| {
                        if let Some(path) = logo_path
                            && let Some(bytes) = load_logo_bytes(path)
                        {
                            let uri = format!("bytes://{path}");
                            h_ui.add(
                                egui::Image::from_bytes(uri, bytes)
                                    .fit_to_exact_size(egui::vec2(16.0, 16.0)),
                            );
                        }
                        let text = egui::RichText::new(label).color(theme.text_inverse);
                        h_ui.label(text);
                    });
                })
                .response
                .on_hover_cursor(egui::CursorIcon::PointingHand);

            if response.interact(egui::Sense::click()).clicked() {
                *current_item = item.clone();
            }
        }
    });
}
