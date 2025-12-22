use crate::ui::app::{Action, LaReviewApp, ReviewAction};
use eframe::egui;

impl LaReviewApp {
    /// Renders the dropdowns for Review and Run selection in the header
    pub(super) fn render_header_selectors(&mut self, ui: &mut egui::Ui) {
        if self.state.domain.reviews.is_empty() {
            return;
        }

        let current_id = self.state.ui.selected_review_id.clone();
        let reviews = self.state.domain.reviews.clone();

        // Find label
        let current_label = current_id
            .as_ref()
            .and_then(|id| reviews.iter().find(|r| &r.id == id))
            .map(|r| r.title.clone())
            .unwrap_or_else(|| "Select review…".to_string());

        // Review Selector
        egui::ComboBox::from_id_salt("review_select")
            .selected_text(egui::RichText::new(current_label).strong())
            .width(200.0)
            .show_ui(ui, |ui| {
                for review in &reviews {
                    let is_selected = current_id.as_deref() == Some(&review.id);
                    if ui.selectable_label(is_selected, &review.title).clicked() {
                        self.dispatch(Action::Review(ReviewAction::SelectReview {
                            review_id: review.id.clone(),
                        }));
                    }
                }
            });
    }
}
