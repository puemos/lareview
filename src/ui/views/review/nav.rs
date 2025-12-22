use crate::ui::app::LaReviewApp;
use crate::ui::spacing;
use crate::ui::theme::current_theme;
use eframe::egui;
use egui_phosphor::regular as icons;

use crate::application::review::ordering::{
    sub_flows_in_display_order, tasks_in_sub_flow_display_order,
};

impl LaReviewApp {
    /// Renders the logic for the Left Panel (Navigation)
    pub(super) fn render_navigation_tree(
        &mut self,
        ui: &mut egui::Ui,
        tasks_by_sub_flow: &std::collections::HashMap<
            Option<String>,
            Vec<crate::domain::ReviewTask>,
        >,
    ) {
        let sub_flows = sub_flows_in_display_order(tasks_by_sub_flow);

        if sub_flows.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new("No tasks loaded")
                        .italics()
                        .color(current_theme().text_muted),
                );
            });
            return;
        }

        ui.spacing_mut().item_spacing = egui::vec2(0.0, spacing::SPACING_SM);
        ui.spacing_mut().indent = 12.0;
        ui.visuals_mut().indent_has_left_vline = true;
        ui.visuals_mut().widgets.noninteractive.bg_stroke =
            egui::Stroke::new(1.0, current_theme().border);

        for (sub_flow_name, tasks) in sub_flows {
            let title = sub_flow_name.as_deref().unwrap_or("UNCATEGORIZED");
            let title_upper = title.to_uppercase();
            let total = tasks.len();
            let finished = tasks.iter().filter(|t| t.status.is_closed()).count();
            let is_done = finished == total && total > 0;

            let header_id = ui.id().with(("sub_flow_collapse", title));

            ui.set_width(ui.available_width());

            egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                header_id,
                true,
            )
            .show_header(ui, |ui| {
                ui.horizontal(|ui| {
                    let mut heading = egui::RichText::new(&title_upper)
                        .family(egui::FontFamily::Proportional)
                        .strong()
                        .size(11.0)
                        .extra_letter_spacing(0.5);

                    if is_done {
                        heading = heading.color(current_theme().text_muted);
                    } else {
                        heading = heading.color(current_theme().text_primary);
                    }

                    ui.label(heading);

                    ui.add_space(spacing::SPACING_XS);

                    let color = if is_done {
                        current_theme().success
                    } else {
                        current_theme().text_muted
                    };

                    let count_text = egui::RichText::new(format!("{}/{}", finished, total))
                        .size(11.0)
                        .color(color);

                    ui.label(count_text);
                });
            })
            .body(|ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing = egui::vec2(0.0, spacing::SPACING_XS);
                for task in tasks_in_sub_flow_display_order(tasks) {
                    self.render_nav_item(ui, task);
                }
            });
        }
    }

    /// Renders a single task item in the sidebar
    pub(super) fn render_nav_item(&mut self, ui: &mut egui::Ui, task: &crate::domain::ReviewTask) {
        let is_selected = self.state.ui.selected_task_id.as_ref() == Some(&task.id);

        let (bg_color, text_color) = if is_selected {
            (
                current_theme().bg_secondary.gamma_multiply(0.5),
                current_theme().text_primary,
            )
        } else {
            (egui::Color32::TRANSPARENT, current_theme().text_muted)
        };

        let (risk_icon, risk_color, risk_label) = match task.stats.risk {
            crate::domain::RiskLevel::High => (
                icons::CARET_CIRCLE_DOUBLE_UP,
                current_theme().destructive,
                "High risk",
            ),
            crate::domain::RiskLevel::Medium => (
                icons::CARET_CIRCLE_UP,
                current_theme().warning,
                "Medium risk",
            ),
            crate::domain::RiskLevel::Low => {
                (icons::CARET_CIRCLE_DOWN, current_theme().accent, "Low risk")
            }
        };

        let mut title_text = egui::RichText::new(&task.title)
            .size(13.0)
            .color(text_color);
        if task.status.is_closed() {
            title_text = title_text.color(current_theme().text_muted).strikethrough();
        }

        // Safety: If available width is less than the margin, we might panic on child allocation.
        let min_needed_width = spacing::SPACING_SM + 4.0;
        if ui.available_width() < min_needed_width {
            return;
        }

        let avail = ui.available_width();
        let response = egui::Frame::NONE
            .fill(bg_color)
            .corner_radius(egui::CornerRadius {
                nw: crate::ui::spacing::RADIUS_MD,
                ne: 0,
                sw: crate::ui::spacing::RADIUS_MD,
                se: 0,
            })
            .inner_margin(egui::Margin {
                left: spacing::SPACING_SM as i8,
                right: 0,
                top: (spacing::SPACING_XS + 1.0) as i8,
                bottom: (spacing::SPACING_XS + 1.0) as i8,
            })
            .show(ui, |ui| {
                ui.set_min_width(avail);
                ui.horizontal(|ui| {
                    // Navigation: risk + crossed title (when closed)
                    ui.label(egui::RichText::new(risk_icon).size(15.0).color(risk_color))
                        .on_hover_text(risk_label);

                    ui.add_space(4.0);

                    ui.add(
                        egui::Label::new(title_text)
                            .truncate()
                            .show_tooltip_when_elided(true),
                    );
                })
                .response
            })
            .response;

        // --- Cursor and Click Logic ---
        let interact_response = response.interact(egui::Sense::click());

        if interact_response.hovered() {
            // Set cursor to pointer (hand) when the item is hovered
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        if interact_response.clicked() {
            self.select_task(task);
        }
    }
}
