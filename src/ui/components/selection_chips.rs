use eframe::egui;
use once_cell::sync::Lazy;
use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::ui::spacing;
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

fn wrap_hint(text: &str) -> Cow<'_, str> {
    if !text.contains(['-', '_', '/', '.', ':']) {
        return Cow::Borrowed(text);
    }

    // Insert zero-width spaces after common separators so Egui can wrap model IDs like
    // `claude-3-5-sonnet` instead of overflowing horizontally.
    let mut out = String::with_capacity(text.len() + 8);
    for ch in text.chars() {
        out.push(ch);
        if matches!(ch, '-' | '_' | '/' | '.' | ':') {
            out.push('\u{200B}');
        }
    }
    Cow::Owned(out)
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
    ui.spacing_mut().item_spacing = egui::vec2(spacing::ITEM_SPACING.0, spacing::ITEM_SPACING.1);

    ui.horizontal_wrapped(|wrap_ui| {
        wrap_ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

        let row_width = {
            let w = wrap_ui.available_width();
            if w.is_finite() {
                w
            } else {
                wrap_ui.clip_rect().width()
            }
        };

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
            let fill = if selected {
                theme.bg_secondary
            } else {
                theme.bg_primary
            };
            let stroke = if selected { theme.brand } else { theme.border };
            let frame = egui::Frame::new()
                .fill(fill)
                .stroke(egui::Stroke::new(1.0, stroke))
                .inner_margin(egui::vec2(spacing::SPACING_SM, spacing::SPACING_XS))
                .corner_radius(egui::CornerRadius::same(255));

            let response = frame
                .show(wrap_ui, |inner_ui| {
                    // allow inner to shrink and wrap
                    inner_ui.set_min_width(0.0);
                    if row_width.is_finite() && row_width > 0.0 {
                        inner_ui.set_max_width((row_width - spacing::SPACING_SM).max(120.0));
                    }

                    inner_ui.horizontal_wrapped(|h_ui| {
                        h_ui.spacing_mut().item_spacing =
                            egui::vec2(spacing::TIGHT_ITEM_SPACING.0, 0.0);

                        if let Some(path) = logo_path
                            && let Some(bytes) = load_logo_bytes(path)
                        {
                            let uri = format!("bytes://{path}");
                            h_ui.add(
                                egui::Image::from_bytes(uri, bytes)
                                    .fit_to_exact_size(egui::vec2(16.0, 16.0)),
                            );
                        }
                        let label = wrap_hint(label);
                        let text = egui::RichText::new(label.as_ref()).color(theme.text_primary);
                        h_ui.add(egui::Label::new(text).wrap());
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
