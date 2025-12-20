use crate::ui::spacing::SPACING_XS;
use crate::ui::theme::current_theme;
use eframe::egui;
use egui_phosphor::regular as icons;

pub struct PopupOption<T> {
    pub label: &'static str,
    pub value: T,
    pub fg: egui::Color32,
    pub icon: Option<&'static str>,
}

pub fn popup_selector<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    id: egui::Id,
    selected: T,
    options: &[PopupOption<T>],
    width: f32,
    enabled: bool,
) -> Option<T> {
    let theme = current_theme();
    let selected_option = options
        .iter()
        .find(|option| option.value == selected)
        .unwrap_or_else(|| {
            options
                .first()
                .expect("Popup selector requires at least one option")
        });

    let is_open = egui::Popup::is_id_open(ui.ctx(), id);
    let button_height = 28.0;

    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, button_height), egui::Sense::click());

    if enabled && response.clicked() {
        egui::Popup::toggle_id(ui.ctx(), id);
    }

    let visuals = ui.style().visuals.clone();
    let bg_fill = if is_open {
        visuals.widgets.open.bg_fill
    } else if response.hovered() && enabled {
        visuals.widgets.hovered.bg_fill
    } else {
        visuals.widgets.inactive.bg_fill
    };

    let stroke = if is_open {
        visuals.widgets.open.bg_stroke
    } else if response.hovered() && enabled {
        visuals.widgets.hovered.bg_stroke
    } else {
        visuals.widgets.inactive.bg_stroke
    };

    ui.painter().rect(
        rect,
        crate::ui::spacing::RADIUS_MD,
        bg_fill,
        stroke,
        egui::StrokeKind::Middle,
    );

    if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand);
    }

    let content_rect = rect.shrink2(egui::vec2(6.0, 0.0));
    ui.scope_builder(egui::UiBuilder::new().max_rect(content_rect), |ui| {
        ui.horizontal_centered(|ui| {
            ui.add_space(2.0);
            let icon = selected_option.icon.unwrap_or(icons::DOT_OUTLINE);
            ui.label(
                egui::RichText::new(icon)
                    .size(12.0)
                    .color(selected_option.fg),
            );
            ui.add_space(6.0);
            ui.add(
                egui::Label::new(
                    egui::RichText::new(selected_option.label).color(theme.text_primary),
                )
                .selectable(false),
            );
            ui.allocate_ui_with_layout(
                ui.available_size(),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    ui.add_space(2.0);
                    ui.add(
                        egui::Label::new(egui::RichText::new("⏷").color(theme.text_disabled))
                            .selectable(false),
                    );
                },
            );
        });
    });

    let mut next = None;
    egui::Popup::new(id, ui.ctx().clone(), rect, ui.layer_id())
        .open_memory(None)
        .show(|ui| {
            egui::ScrollArea::vertical()
                .max_height(240.0)
                .show(ui, |ui| {
                    ui.set_width(width);
                    let item_spacing_x = ui.spacing().item_spacing.x;
                    ui.spacing_mut().item_spacing = egui::vec2(item_spacing_x, 0.0);

                    let item_height = 24.0;
                    let item_gap = SPACING_XS;
                    let row_height = item_height + item_gap;
                    let item_inset = item_gap * 0.5;
                    let selected_bg = theme.brand.gamma_multiply(0.25);
                    let selected_text = theme.text_primary;

                    for option in options {
                        let is_selected = option.value == selected;
                        let (item_row_rect, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), row_height),
                            egui::Sense::hover(),
                        );
                        let item_rect = item_row_rect.shrink2(egui::vec2(0.0, item_inset));
                        let item_id = ui.make_persistent_id((id, option.label));
                        let mut item_response =
                            ui.interact(item_rect, item_id, egui::Sense::click());

                        let mut item_bg = egui::Color32::TRANSPARENT;
                        let mut text_color = theme.text_primary;

                        if is_selected {
                            item_bg = selected_bg;
                            text_color = selected_text;
                        } else if item_response.hovered() {
                            item_bg = theme.bg_secondary;
                        }

                        ui.painter()
                            .rect_filled(item_rect, crate::ui::spacing::RADIUS_MD, item_bg);

                        let content_rect = item_rect.shrink2(egui::vec2(8.0, 4.0));
                        let item_ui = egui::UiBuilder::new().max_rect(content_rect);
                        ui.scope_builder(item_ui, |ui| {
                            ui.style_mut().interaction.selectable_labels = false;
                            ui.horizontal_centered(|ui| {
                                let icon = option.icon.unwrap_or(icons::DOT_OUTLINE);
                                ui.label(egui::RichText::new(icon).size(12.0).color(option.fg));
                                ui.add_space(6.0);
                                let label = if is_selected {
                                    egui::RichText::new(option.label).color(text_color).strong()
                                } else {
                                    egui::RichText::new(option.label).color(text_color)
                                };
                                ui.add(egui::Label::new(label).selectable(false));
                            });
                        });

                        if enabled {
                            item_response =
                                item_response.on_hover_cursor(egui::CursorIcon::PointingHand);
                        }

                        if item_response.clicked() {
                            next = Some(option.value);
                            egui::Popup::close_id(ui.ctx(), id);
                        }
                    }
                });
        });

    next
}
