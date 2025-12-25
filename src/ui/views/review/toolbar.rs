use crate::domain::Review;
use crate::ui::app::ReviewAction;
use crate::ui::theme::Theme;
use crate::ui::typography;
use eframe::egui;
use egui_phosphor::regular;

/// Renders the dropdowns for Review and Run selection in the header
pub(crate) fn render_header_selectors(
    ui: &mut egui::Ui,
    reviews: &[Review],
    selected_review_id: Option<&String>,
    theme: &Theme,
) -> Option<ReviewAction> {
    if reviews.is_empty() {
        return None;
    }

    // Find label
    let current_label = selected_review_id
        .and_then(|id| reviews.iter().find(|r| &r.id == id))
        .map(|r| r.title.clone())
        .unwrap_or_else(|| "Select review…".to_string());

    let mut action_out = None;

    // Review Selector - Custom "Title" Button
    let id = ui.make_persistent_id("review_selector");
    let is_open = egui::Popup::is_id_open(ui.ctx(), id);

    ui.scope(|ui| {
        // Visuals overrides
        let visuals = ui.visuals_mut();
        visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
        visuals.widgets.hovered.weak_bg_fill = egui::Color32::TRANSPARENT;
        visuals.widgets.active.weak_bg_fill = egui::Color32::TRANSPARENT;
        visuals.widgets.open.weak_bg_fill = egui::Color32::TRANSPARENT;

        visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
        visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
        visuals.widgets.open.bg_stroke = egui::Stroke::NONE;

        let text_content = format!("{} {}", current_label, regular::CARET_UP_DOWN);
        let mut text = typography::body(text_content).size(16.0);

        // 1. Calculate size
        let galley = ui.painter().layout_no_wrap(
            text.text().to_string(),
            typography::body_font(16.0),
            theme.text_primary,
        );

        // 2. Allocate
        let (rect, response) = ui.allocate_exact_size(galley.size(), egui::Sense::click());
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &current_label)
        });

        // 3. Determine color
        let text_color = if response.hovered() || is_open {
            theme.brand
        } else {
            theme.text_primary
        };

        text = text.color(text_color);

        // 4. Draw
        ui.painter().galley(
            rect.min,
            ui.painter().layout_no_wrap(
                text.text().to_string(),
                typography::body_font(16.0),
                text_color,
            ),
            text_color,
        );

        if response.clicked() {
            egui::Popup::toggle_id(ui.ctx(), id);
        }

        egui::Popup::new(id, ui.ctx().clone(), rect, ui.layer_id())
            .open_memory(None)
            .show(|ui| {
                ui.set_min_width(300.0);
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 8.0);

                // Sort reviews: Selected first, then by date desc
                let mut sorted_reviews: Vec<&Review> = reviews.iter().collect();
                sorted_reviews.sort_by(|a, b| {
                    let a_selected = selected_review_id == Some(&a.id);
                    let b_selected = selected_review_id == Some(&b.id);
                    if a_selected && !b_selected {
                        std::cmp::Ordering::Less
                    } else if !a_selected && b_selected {
                        std::cmp::Ordering::Greater
                    } else {
                        b.created_at.cmp(&a.created_at)
                    }
                });

                for review in sorted_reviews {
                    let is_selected = selected_review_id == Some(&review.id);

                    // Custom list item layout
                    let desired_size = egui::vec2(ui.available_width(), 32.0);
                    let (rect, response) =
                        ui.allocate_exact_size(desired_size, egui::Sense::click());

                    let is_hovered = response.hovered();
                    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

                    // Background (Only for selected when not hovered)
                    if !is_hovered && is_selected {
                        ui.painter().rect_filled(rect, 4.0, theme.bg_tertiary);
                    }

                    // Content
                    let content_rect = rect.shrink(8.0);
                    let text_color = if is_selected || is_hovered {
                        theme.brand
                    } else {
                        theme.text_primary
                    };

                    let font_id = egui::FontId::proportional(14.0);
                    let mut job = egui::text::LayoutJob::single_section(
                        review.title.clone(),
                        egui::TextFormat {
                            font_id,
                            color: text_color,
                            ..Default::default()
                        },
                    );
                    job.wrap.max_width = content_rect.width();
                    job.wrap.max_rows = 1;
                    job.wrap.break_anywhere = true;

                    let galley = ui.painter().layout_job(job);
                    ui.painter().galley(content_rect.min, galley, text_color);

                    if response.clicked() {
                        action_out = Some(ReviewAction::SelectReview {
                            review_id: review.id.clone(),
                        });
                        egui::Popup::close_id(ui.ctx(), id);
                    }
                }
            });
    });

    action_out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ReviewSource;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    #[test]
    fn test_render_header_selectors_empty() {
        let reviews = vec![];
        let mut harness = Harness::new_ui(|ui| {
            render_header_selectors(ui, &reviews, None, &crate::ui::theme::current_theme());
        });
        harness.run();
    }

    #[test]
    fn test_render_header_selectors_no_selection() {
        let reviews = vec![Review {
            id: "rev1".into(),
            title: "Test Review".into(),
            summary: None,
            source: ReviewSource::DiffPaste {
                diff_hash: "h".into(),
            },
            active_run_id: None,
            created_at: "now".into(),
            updated_at: "now".into(),
        }];
        let mut harness = Harness::new_ui(|ui| {
            crate::ui::app::LaReviewApp::setup_fonts(ui.ctx());
            render_header_selectors(ui, &reviews, None, &crate::ui::theme::current_theme());
        });
        harness.run_steps(5);
        harness
            .get_all_by_role(egui::accesskit::Role::Button)
            .into_iter()
            .find(|n| format!("{:?}", n).contains("Select review"))
            .expect("Selector not found");
    }

    #[test]
    fn test_render_header_selectors_with_selection() {
        let reviews = vec![Review {
            id: "rev1".into(),
            title: "Selected Review".into(),
            summary: None,
            source: ReviewSource::DiffPaste {
                diff_hash: "h".into(),
            },
            active_run_id: None,
            created_at: "now".into(),
            updated_at: "now".into(),
        }];
        let mut harness = Harness::new_ui(|ui| {
            crate::ui::app::LaReviewApp::setup_fonts(ui.ctx());
            render_header_selectors(
                ui,
                &reviews,
                Some(&"rev1".to_string()),
                &crate::ui::theme::current_theme(),
            );
        });
        harness.run_steps(5);
        harness
            .get_all_by_role(egui::accesskit::Role::Button)
            .into_iter()
            .find(|n| format!("{:?}", n).contains("Selected Review"))
            .expect("Review label not found");
    }

    #[test]
    fn test_render_header_selectors_multiple() {
        let reviews = vec![
            Review {
                id: "rev1".into(),
                title: "Review 1".into(),
                summary: None,
                source: ReviewSource::DiffPaste {
                    diff_hash: "h1".into(),
                },
                active_run_id: None,
                created_at: "2024-01-01T00:00:00Z".into(),
                updated_at: "2024-01-01T00:00:00Z".into(),
            },
            Review {
                id: "rev2".into(),
                title: "Review 2".into(),
                summary: None,
                source: ReviewSource::DiffPaste {
                    diff_hash: "h2".into(),
                },
                active_run_id: None,
                created_at: "2024-01-02T00:00:00Z".into(),
                updated_at: "2024-01-02T00:00:00Z".into(),
            },
        ];
        let mut harness = Harness::new_ui(|ui| {
            crate::ui::app::LaReviewApp::setup_fonts(ui.ctx());
            render_header_selectors(
                ui,
                &reviews,
                Some(&"rev1".to_string()),
                &crate::ui::theme::current_theme(),
            );
        });
        harness.run_steps(5);
    }
}
