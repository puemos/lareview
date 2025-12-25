use crate::domain::LinkedRepo;
use crate::ui::icons;
use crate::ui::spacing::SPACING_XS;
use crate::ui::theme::current_theme;
use crate::ui::typography;
use eframe::egui;
use egui_phosphor::regular;

pub fn repo_selector(
    ui: &mut egui::Ui,
    selected_repo_id: &mut Option<String>,
    repos: &[LinkedRepo],
) {
    let selected_repo = selected_repo_id
        .as_ref()
        .and_then(|id| repos.iter().find(|r| &r.id == id));

    let selected_label = selected_repo
        .map(|r| r.name.clone())
        .unwrap_or_else(|| "No Repo Context".to_string());

    ui.push_id("repo_selector_combo", |ui| {
        let id = ui.make_persistent_id("repo_selector_popup");
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

        let stroke = if is_open {
            visuals.widgets.open.bg_stroke
        } else if response.hovered() {
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

        // Draw Content
        let content_rect = rect.shrink2(egui::vec2(8.0, 0.0));
        let ui_builder = egui::UiBuilder::new().max_rect(content_rect);
        ui.scope_builder(ui_builder, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(2.0);

                // Icon
                ui.label(
                    typography::body(icons::VIEW_REPOS)
                        .size(16.0)
                        .color(current_theme().text_muted),
                );
                ui.add_space(6.0);

                // Text
                ui.add(
                    egui::Label::new(typography::body(selected_label).color(
                        if selected_repo_id.is_some() {
                            current_theme().text_primary
                        } else {
                            current_theme().text_disabled
                        },
                    ))
                    .selectable(false),
                );

                // Chevron
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(2.0);
                    ui.label(
                        typography::body(regular::CARET_UP_DOWN)
                            .color(current_theme().text_disabled),
                    );
                });
            });
        });

        // 2. Draw Popup
        egui::Popup::new(id, ui.ctx().clone(), rect, ui.layer_id())
            .open_memory(None)
            .show(|ui| {
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        ui.set_width(width);
                        let item_spacing_x = ui.spacing().item_spacing.x;
                        ui.spacing_mut().item_spacing = egui::vec2(item_spacing_x, 0.0);

                        let item_height = 24.0;
                        let item_gap = SPACING_XS;
                        let row_height = item_height + item_gap;
                        let item_inset = item_gap * 0.5;
                        let selected_bg = current_theme().brand.gamma_multiply(0.25);
                        let selected_text = current_theme().text_primary;

                        // Option: None
                        let none_selected = selected_repo_id.is_none();
                        let (none_row_rect, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), row_height),
                            egui::Sense::hover(),
                        );
                        let none_rect = none_row_rect.shrink2(egui::vec2(0.0, item_inset));
                        let none_id = ui.make_persistent_id("repo_item_none");
                        let none_response = ui
                            .interact(none_rect, none_id, egui::Sense::click())
                            .on_hover_cursor(egui::CursorIcon::PointingHand);

                        let mut none_bg = egui::Color32::TRANSPARENT;
                        let mut none_text = current_theme().text_disabled;

                        if none_selected {
                            none_bg = selected_bg;
                            none_text = selected_text;
                        } else if none_response.hovered() {
                            none_bg = current_theme().bg_secondary;
                        }

                        ui.painter()
                            .rect_filled(none_rect, crate::ui::spacing::RADIUS_MD, none_bg);

                        let none_content = none_rect.shrink2(egui::vec2(8.0, 4.0));
                        let none_ui = egui::UiBuilder::new().max_rect(none_content);
                        ui.scope_builder(none_ui, |ui| {
                            ui.style_mut().interaction.selectable_labels = false;
                            ui.horizontal_centered(|ui| {
                                let label =
                                    typography::body("No Repository Context").color(none_text);
                                ui.add(egui::Label::new(label).selectable(false));
                            });
                        });

                        if none_response.clicked() {
                            *selected_repo_id = None;
                            egui::Popup::close_id(ui.ctx(), id);
                        }

                        for repo in repos {
                            let is_selected = selected_repo_id.as_ref() == Some(&repo.id);
                            let (item_row_rect, _) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), row_height),
                                egui::Sense::hover(),
                            );
                            let item_rect = item_row_rect.shrink2(egui::vec2(0.0, item_inset));
                            let item_id = ui.make_persistent_id(format!("repo_item_{}", repo.id));
                            let item_response = ui
                                .interact(item_rect, item_id, egui::Sense::click())
                                .on_hover_cursor(egui::CursorIcon::PointingHand);

                            let mut item_bg = egui::Color32::TRANSPARENT;

                            if is_selected {
                                item_bg = selected_bg;
                            } else if item_response.hovered() {
                                item_bg = current_theme().bg_secondary;
                            }

                            ui.painter().rect_filled(
                                item_rect,
                                crate::ui::spacing::RADIUS_MD,
                                item_bg,
                            );

                            let content_rect = item_rect.shrink2(egui::vec2(8.0, 4.0));
                            let item_ui = egui::UiBuilder::new().max_rect(content_rect);
                            ui.scope_builder(item_ui, |ui| {
                                ui.style_mut().interaction.selectable_labels = false;
                                ui.horizontal_centered(|ui| {
                                    let label = if is_selected {
                                        typography::body(&repo.name)
                                            .color(current_theme().text_primary)
                                    } else {
                                        typography::body(&repo.name)
                                            .color(current_theme().text_tertiary)
                                    };
                                    ui.add(egui::Label::new(label).selectable(false));
                                });
                            });

                            if item_response.clicked() {
                                *selected_repo_id = Some(repo.id.clone());
                                egui::Popup::close_id(ui.ctx(), id);
                            }
                        }
                    });
            });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::LinkedRepo;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_repo_selector_rendering() {
        let selected = Arc::new(Mutex::new(None));
        let selected_clone = selected.clone();
        let repos = vec![LinkedRepo {
            id: "repo-1".into(),
            name: "Repo 1".into(),
            path: PathBuf::from("/path/1"),
            remotes: vec![],
            created_at: "".into(),
        }];

        let mut harness = Harness::new(move |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut selected_guard = selected_clone.lock().unwrap();
                repo_selector(ui, &mut selected_guard, &repos);
            });
        });

        harness.run_steps(1);
        harness.get_by_label("No Repo Context");
    }

    #[test]
    fn test_repo_selector_selection() {
        let selected = Arc::new(Mutex::new(None));
        let selected_clone = selected.clone();
        let repos = vec![LinkedRepo {
            id: "repo-1".into(),
            name: "Repo 1".into(),
            path: PathBuf::from("/path/1"),
            remotes: vec![],
            created_at: "".into(),
        }];

        let mut harness = Harness::new(move |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut selected_guard = selected_clone.lock().unwrap();
                repo_selector(ui, &mut selected_guard, &repos);
            });
        });

        harness.run_steps(1);
        harness.get_by_label("No Repo Context").click();
        harness.run_steps(1);

        // Popup is open, should see "Repo 1"
        harness.get_by_label("Repo 1").click();
        harness.run_steps(1);

        assert_eq!(*selected.lock().unwrap(), Some("repo-1".to_string()));
    }
}
