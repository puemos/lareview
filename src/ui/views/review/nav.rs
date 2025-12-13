use crate::ui::app::LaReviewApp;
use crate::ui::spacing;
use catppuccin_egui::MOCHA;
use eframe::egui;
use egui_phosphor::regular as icons;

impl LaReviewApp {
    /// Renders a single task item in the sidebar
    pub(super) fn render_nav_item(&mut self, ui: &mut egui::Ui, task: &crate::domain::ReviewTask) {
        let is_selected = self.state.selected_task_id.as_ref() == Some(&task.id);

        let (bg_color, text_color) = if is_selected {
            (MOCHA.surface1, MOCHA.text)
        } else {
            (egui::Color32::TRANSPARENT, MOCHA.subtext0)
        };

        let (risk_icon, risk_color, risk_label) = match task.stats.risk {
            crate::domain::RiskLevel::High => {
                (icons::CARET_CIRCLE_DOUBLE_UP, MOCHA.red, "High risk")
            }
            crate::domain::RiskLevel::Medium => {
                (icons::CARET_CIRCLE_UP, MOCHA.yellow, "Medium risk")
            }
            crate::domain::RiskLevel::Low => (icons::CARET_CIRCLE_DOWN, MOCHA.blue, "Low risk"),
        };

        let mut title_text = egui::RichText::new(&task.title)
            .size(13.0)
            .color(text_color);
        if task.status.is_closed() {
            title_text = title_text.color(MOCHA.subtext0).strikethrough();
        }

        let response = egui::Frame::NONE
            .fill(bg_color)
            .corner_radius(4.0)
            .inner_margin(egui::Margin::symmetric(
                spacing::SPACING_SM as i8,
                (spacing::SPACING_XS + 2.0) as i8,
            ))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    // Navigation: risk + crossed title (when closed)
                    ui.label(egui::RichText::new(risk_icon).size(16.0).color(risk_color))
                        .on_hover_text(risk_label);

                    ui.add_space(6.0); // Keep 6.0 as this is a custom spacing value

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
