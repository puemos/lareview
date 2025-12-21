use crate::ui::app::LaReviewApp;
use crate::ui::spacing;
use crate::ui::theme::current_theme;
use eframe::egui;
use egui_phosphor::regular as icons;

impl LaReviewApp {
    /// Renders a single task item in the sidebar
    pub(super) fn render_nav_item(&mut self, ui: &mut egui::Ui, task: &crate::domain::ReviewTask) {
        let is_selected = self.state.selected_task_id.as_ref() == Some(&task.id);

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
