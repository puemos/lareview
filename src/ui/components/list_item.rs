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
    checkbox: Option<bool>,
    action: Option<Box<dyn FnOnce() + 'a>>,
    inner_margin: Option<egui::Margin>,
}

impl<'a> ListItem<'a> {
    pub fn new(title: egui::RichText) -> Self {
        Self {
            title,
            status_icon: None,
            subtitle: None,
            metadata: None,
            selected: false,
            checkbox: None,
            action: None,
            inner_margin: None,
        }
    }

    pub fn inner_margin(mut self, margin: egui::Margin) -> Self {
        self.inner_margin = Some(margin);
        self
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

    pub fn checkbox(mut self, checked: &mut bool) -> Self {
        self.checkbox = Some(*checked);
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

        // We use an egui::Frame to allocate the background area for the item.
        // This ensures the background color covers the entire inner area,
        // including the specified padding/margin.

        let margin = self
            .inner_margin
            .unwrap_or(egui::Margin::same(spacing::SPACING_SM as i8));

        let frame = egui::Frame::NONE
            .inner_margin(margin)
            .fill(bg_color)
            .corner_radius(spacing::RADIUS_MD);

        // Frame::show returns a response for the inner content. We manually
        // manage the interactive sense to make the entire frame clickable.

        let response = frame
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    // Align icons with the first line of text (the title).

                    if let Some((icon, color)) = self.status_icon {
                        // We want the icon to be aligned with the first line of text.
                        // A simple way is to use `ui.with_layout` to align to TOP-LEFT.
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                            // Apply a small vertical offset if fonts differ significantly
                            // in size to achieve optical centering.
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

        // Interaction handling
        let response = response.interact(egui::Sense::click());

        // Note: The hover effect in this simplified `show` implementation is handled
        // after drawing. For perfect hover states without frame lag, use `show_with_bg`.

        // Handle user interaction feedback
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
        let mut action = self.action;
        let mut checkbox_clicked = false;

        // Allocate a placeholder for the background shape to be painted after
        // inner layout and interaction ares are determined.
        let bg_shape_idx = ui.painter().add(egui::Shape::Noop);
        let title_text = self.title.text().to_string();

        let margin = self
            .inner_margin
            .unwrap_or(egui::Margin::same(spacing::SPACING_SM as i8));

        let frame = egui::Frame::NONE
            .inner_margin(margin)
            .fill(egui::Color32::TRANSPARENT);

        let response = frame
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    // Align Top
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                        if let Some(checked) = self.checkbox {
                            ui.vertical(|ui| {
                                ui.add_space(2.0); // Optical alignment with title text
                                let icon = if checked {
                                    crate::ui::icons::ICON_CHECK_SQUARE
                                } else {
                                    crate::ui::icons::ICON_SQUARE
                                };
                                let checkbox_resp = ui.add(
                                    egui::Label::new(
                                        typography::body(icon)
                                            .size(18.0)
                                            .color(egui::Color32::WHITE),
                                    )
                                    .sense(egui::Sense::click()),
                                );
                                checkbox_resp.widget_info(|| {
                                    egui::WidgetInfo::selected(
                                        egui::WidgetType::Checkbox,
                                        checked,
                                        true,
                                        title_text.clone(),
                                    )
                                });
                                if checkbox_resp.clicked() {
                                    checkbox_clicked = true;
                                }
                            });
                            ui.add_space(spacing::SPACING_SM);
                        }

                        if let Some((icon, color)) = self.status_icon {
                            // Status Icon Column
                            ui.vertical(|ui| {
                                // Apply small vertical offset to center icon with adjacent text
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

        let triggered = checkbox_clicked || response.clicked();
        if triggered && let Some(action) = action.take() {
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

    #[test]
    fn test_list_item_checkbox_click() {
        use std::sync::{Arc, Mutex};
        let changed = Arc::new(Mutex::new(false));
        let changed_clone = changed.clone();

        let mut harness = egui_kittest::Harness::new(move |ctx| {
            crate::ui::app::LaReviewApp::setup_fonts(ctx);
            egui::CentralPanel::default().show(ctx, |ui| {
                let theme = crate::ui::theme::current_theme();
                let changed_clone = changed_clone.clone();
                let mut checked = false;
                let item = ListItem::new(egui::RichText::new("Check Me"))
                    .checkbox(&mut checked)
                    .action(move || {
                        let mut guard = changed_clone.lock().unwrap();
                        *guard = true;
                    });
                item.show_with_bg(ui, &theme);
            });
        });

        harness.run_steps(1);
        // Find the checkbox and click it
        harness.get_by_role(egui::accesskit::Role::CheckBox).click();
        harness.run_steps(1);

        assert!(*changed.lock().unwrap());
    }
}
