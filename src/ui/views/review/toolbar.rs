use crate::domain::Review;
use crate::ui::app::ReviewAction;
use crate::ui::theme::Theme;
use eframe::egui;

/// Renders the dropdowns for Review and Run selection in the header
pub(crate) fn render_header_selectors(
    ui: &mut egui::Ui,
    reviews: &[Review],
    selected_review_id: Option<&String>,
    _theme: &Theme,
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

    // Review Selector
    egui::ComboBox::from_id_salt("review_select")
        .selected_text(egui::RichText::new(current_label).strong())
        .width(200.0)
        .show_ui(ui, |ui| {
            for review in reviews {
                let is_selected = selected_review_id == Some(&review.id);
                if ui.selectable_label(is_selected, &review.title).clicked() {
                    action_out = Some(ReviewAction::SelectReview {
                        review_id: review.id.clone(),
                    });
                }
            }
        });

    action_out
}
