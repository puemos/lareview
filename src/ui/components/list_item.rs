use crate::ui::spacing;
use crate::ui::theme::Theme;
use crate::ui::typography;
use eframe::egui;

pub struct ListItem<'a> {
    title: egui::RichText,
    status_icon: Option<(&'static str, egui::Color32)>,
    subtitle: Option<egui::RichText>,
    metadata: Option<egui::WidgetText>,
    selected: bool,
    action: Option<Box<dyn FnOnce() + 'a>>,
}

impl<'a> ListItem<'a> {
    pub fn new(title: egui::RichText) -> Self {
        Self {
            title,
            status_icon: None,
            subtitle: None,
            metadata: None,
            selected: false,
            action: None,
        }
    }

    pub fn status_icon(mut self, icon: &'static str, color: egui::Color32) -> Self {
        self.status_icon = Some((icon, color));
        self
    }

    pub fn subtitle(mut self, subtitle: egui::RichText) -> Self {
        self.subtitle = Some(subtitle);
        self
    }

    pub fn metadata(mut self, metadata: impl Into<egui::WidgetText>) -> Self {
        self.metadata = Some(metadata.into());
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn action(mut self, action: impl FnOnce() + 'a) -> Self {
        self.action = Some(Box::new(action));
        self
    }

    pub fn show(self, ui: &mut egui::Ui, theme: &Theme) -> egui::Response {
        let is_selected = self.selected;

        let bg_color = if is_selected {
            theme.bg_secondary
        } else {
            egui::Color32::TRANSPARENT
        };

        // We need to allocate space first to handle hover state for background
        // But since we want the background to cover the whole area including padding,
        // we use a Frame.

        let frame = egui::Frame::NONE
            .inner_margin(spacing::SPACING_SM)
            .fill(bg_color)
            .corner_radius(spacing::RADIUS_MD);

        // We need to handle interaction. The Frame doesn't directly return an Interact/Response
        // that covers the whole area easily unless we treat the content as the interactive part.
        // However, standard egui pattern for selectable list items often involves allocating the rect first
        // or using `ui.interact`.

        // Let's use `frame.show` and interact with the inner response,
        // but we might want the hover effect to be on the *frame* area.
        // A common trick is to use `ui.scope` or just rely on the fact that if we click the content, it works.
        // To make the WHOLE frame clickable and hoverable (changing bg), we can use a button-like behavior
        // or manually paint the background on hover.

        // Let's manually handle the background on hover to match `thread_list.rs` behavior logic
        // which was: draw transparent rect, check hover, paint rect if hovered.

        // Allocate space? No, let's just use the Frame, get the response, and then check interactions.

        let response = frame
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    // Top-align the icon with the title
                    // To do this, we can't just center vertically.
                    // We'll use a specific vertical alignment or just lay them out.

                    if let Some((icon, color)) = self.status_icon {
                        // We want the icon to be aligned with the first line of text.
                        // A simple way is to use `ui.with_layout` to align to TOP-LEFT.
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                            // Add a little vertical spacing if needed to visually center with text cap height
                            // But usually straight top align is fine if fonts match.
                            // Let's add a tiny bit of spacing if the font size difference is large.
                            // Assuming standard icon size 16.0 and text 13.0/14.0.
                            ui.label(typography::body(icon).color(color).size(16.0));
                        });

                        ui.add_space(spacing::SPACING_SM);
                    }

                    ui.vertical(|ui| {
                        ui.label(self.title);

                        if let Some(sub) = self.subtitle {
                            ui.add_space(spacing::SPACING_SM);
                            ui.label(sub);
                        }

                        if let Some(meta) = self.metadata {
                            ui.add_space(spacing::SPACING_SM);
                            ui.label(meta);
                        }
                    });
                });
            })
            .response;

        // Interaction
        let response = response.interact(egui::Sense::click());

        // Hover Effect
        if response.hovered() && !is_selected {
            // Paint the hover background behind the content
            // We need to be careful about z-index or just paint it.
            // Since we already painted a transparent/selected background in Frame,
            // we can paint over it or use the painter to paint *before* if we were allocating manually.
            // But `frame.fill` happened already.

            // `thread_list.rs` did: `bg_shape_idx = ui.painter().add(egui::Shape::Noop)` BEFORE content
            // then `ui.painter().set(...)` AFTER content if hovered.
            // That is a cleaner way to handle "hover style on top" or "replace background".

            // However, since we are inside `show`, we can't easily modify the *previous* frame's fill
            // unless we used that `Shape::Noop` trick.

            // Simplified approach for now:
            // Just rely on egui's immediate mode: checking hover *after* drawing is too late for *this* frame's background
            // unless we use layers or the shape trick.
            // Given I want to preserve the `thread_list` logic which worked well:
        }

        // Let's rewrite `show` to use the Shape trick for perfect hover states.

        // Re-do show logic

        // 1. Get rect/layout
        // But we have dynamic content heavily dependent on width.
        // So we really want `frame.show`.

        // Let's stick to the `thread_list.rs` implementation pattern exactly.

        ui.ctx().set_cursor_icon(if response.hovered() {
            egui::CursorIcon::PointingHand
        } else {
            egui::CursorIcon::Default
        });

        // Handle click
        if response.clicked()
            && let Some(action) = self.action
        {
            action();
        }

        // Return response so caller can use it if needed (e.g. for context menu)
        response
    }
}

// Redefining show with the Shape trick for background
impl<'a> ListItem<'a> {
    pub fn show_with_bg(self, ui: &mut egui::Ui, theme: &Theme) -> egui::Response {
        let is_selected = self.selected;

        // Placeholder for background
        let bg_shape_idx = ui.painter().add(egui::Shape::Noop);
        let title_text = self.title.text().to_string();

        let frame = egui::Frame::NONE
            .inner_margin(spacing::SPACING_SM)
            // We do NOT set fill here, we handle it manually
            .fill(egui::Color32::TRANSPARENT);

        let response = frame
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    // Align Top
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                        if let Some((icon, color)) = self.status_icon {
                            // Icon Column
                            ui.vertical(|ui| {
                                // ensure icon is positioned correctly, maybe add small Y offset to optical center with text
                                ui.label(typography::body(icon).color(color).size(16.0));
                            });
                            ui.add_space(spacing::SPACING_SM);
                        }

                        // Text Column
                        ui.vertical(|ui| {
                            ui.label(self.title);

                            if let Some(sub) = self.subtitle {
                                ui.add_space(spacing::SPACING_SM);
                                ui.label(sub);
                            }

                            if let Some(meta) = self.metadata {
                                ui.add_space(spacing::SPACING_SM);
                                ui.label(meta);
                            }
                        });
                    });
                });
            })
            .response;

        let response = response.interact(egui::Sense::click());

        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, is_selected, title_text.clone())
        });

        // Hover logic
        let mut bg_color = if is_selected {
            theme.bg_secondary
        } else {
            egui::Color32::TRANSPARENT
        };

        if response.hovered() && !is_selected {
            bg_color = theme.bg_secondary; // Or a slightly lighter/different shade if needed, but usually same as active or slightly simpler
        }

        if bg_color != egui::Color32::TRANSPARENT {
            ui.painter().set(
                bg_shape_idx,
                egui::Shape::rect_filled(
                    response.rect, // Fill the whole allocated rect
                    spacing::RADIUS_MD,
                    bg_color,
                ),
            );
        }

        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        if response.clicked()
            && let Some(action) = self.action
        {
            action();
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::current_theme;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    #[test]

    fn test_list_item_rendering() {
        let mut harness = Harness::new(|ctx| {
            crate::ui::app::LaReviewApp::setup_fonts(ctx);

            egui::CentralPanel::default().show(ctx, |ui| {
                let theme = current_theme();

                let item = ListItem::new(egui::RichText::new("Test Item"))
                    .subtitle(egui::RichText::new("Subtitle"))
                    .metadata("Meta");

                item.show_with_bg(ui, &theme);
            });
        });

        harness.run_steps(2);

        harness.get_by_role(egui::accesskit::Role::Button);

        harness.get_by_label("Subtitle");

        harness.get_by_label("Meta");
    }

    #[test]

    fn test_list_item_click() {
        use std::sync::{Arc, Mutex};

        let clicked = Arc::new(Mutex::new(false));

        let clicked_clone = clicked.clone();

        let mut harness = Harness::new(move |ctx| {
            crate::ui::app::LaReviewApp::setup_fonts(ctx);

            egui::CentralPanel::default().show(ctx, |ui| {
                let theme = current_theme();

                let clicked_clone = clicked_clone.clone();

                let item = ListItem::new(egui::RichText::new("Click Me")).action(move || {
                    let mut guard = clicked_clone.lock().unwrap();

                    *guard = true;
                });

                item.show_with_bg(ui, &theme);
            });
        });

        harness.run_steps(1);

        harness.get_by_role(egui::accesskit::Role::Button).click();

        harness.run_steps(1);

        assert!(*clicked.lock().unwrap());
    }
}
