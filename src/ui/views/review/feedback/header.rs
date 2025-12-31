use crate::domain::{Feedback, FeedbackImpact, ReviewStatus};
use crate::ui::app::ReviewAction;
use crate::ui::components::pills::pill_action_button;
use crate::ui::components::{PopupOption, popup_selector};
use crate::ui::theme::Theme;
use crate::ui::{spacing, typography};
use eframe::egui;
use egui::Color32;

pub(crate) fn render_feedback_header(
    ui: &mut egui::Ui,
    feedback: Option<&Feedback>,
    link_url: Option<String>,
    theme: &Theme,
    task_id: &str,
    draft_key: &str,
) -> Option<ReviewAction> {
    let ctx = ui.ctx().clone();
    let feedback_title_draft = crate::ui::app::ui_memory::get_ui_memory(&ctx)
        .feedback_drafts
        .get(draft_key)
        .map(|d| d.title.clone())
        .unwrap_or_else(|| feedback.map(|f| f.title.clone()).unwrap_or_default());
    let mut action_out = None;

    let feedback_id = feedback.map(|t| t.id.clone());
    let existing_title = feedback.map(|t| t.title.clone()).unwrap_or_default();
    let can_edit_feedback = feedback_id.is_some();

    // Row 1: title only
    ui.horizontal(|ui| {
        let mut edit_text = feedback_title_draft.to_string();
        let response = ui
            .scope(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut edit_text)
                        .hint_text("Feedback Title")
                        .desired_width(ui.available_width())
                        .text_color(Color32::WHITE)
                        .text_color_opt(Some(theme.text_muted))
                        .font(typography::body_font(16.0))
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                )
            })
            .inner;

        if response.changed() {
            crate::ui::app::ui_memory::with_ui_memory_mut(&ctx, |mem| {
                mem.feedback_drafts
                    .entry(draft_key.to_string())
                    .or_default()
                    .title = edit_text.clone();
            });
        }

        if response.lost_focus()
            && edit_text != existing_title
            && let Some(feedback_id) = feedback_id.clone()
        {
            action_out = Some(ReviewAction::UpdateFeedbackTitle {
                feedback_id,
                title: edit_text.clone(),
            });
        }
    });

    ui.add_space(spacing::SPACING_SM);

    // Row 2: actions (left aligned)
    ui.horizontal(|ui| {
        let status_width = 120.0;
        let impact_width = 150.0;

        // Status selector
        let status = feedback.map(|t| t.status).unwrap_or(ReviewStatus::Todo);
        let status_choices = status_options(theme);
        if let Some(next_status) = popup_selector(
            ui,
            ui.make_persistent_id(("feedback_status_popup", task_id, &feedback_id)),
            status,
            &status_choices,
            status_width,
            can_edit_feedback,
        ) && let Some(feedback_id) = feedback_id.clone()
        {
            action_out = Some(ReviewAction::UpdateFeedbackStatus {
                feedback_id,
                status: next_status,
            });
        }

        ui.add_space(spacing::SPACING_SM);

        // Impact selector
        let impact = feedback
            .map(|t| t.impact)
            .unwrap_or(FeedbackImpact::Nitpick);
        let impact_choices = impact_options(theme);
        if let Some(next_impact) = popup_selector(
            ui,
            ui.make_persistent_id(("feedback_impact_popup", task_id, &feedback_id)),
            impact,
            &impact_choices,
            impact_width,
            can_edit_feedback,
        ) && let Some(feedback_id) = feedback_id.clone()
        {
            action_out = Some(ReviewAction::UpdateFeedbackImpact {
                feedback_id,
                impact: next_impact,
            });
        }

        ui.add_space(spacing::SPACING_SM);

        if let Some(url) = link_url.as_ref()
            && pill_action_button(
                ui,
                crate::ui::icons::ACTION_OPEN_WINDOW,
                "See on GitHub",
                true,
                theme.brand,
            )
            .on_hover_text("Open in Browser")
            .clicked()
        {
            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
        }

        // Delete
        if let Some(feedback_id) = feedback_id.clone()
            && pill_action_button(
                ui,
                crate::ui::icons::ACTION_DELETE,
                "Delete",
                true,
                theme.destructive,
            )
            .on_hover_text("Delete Feedback")
            .clicked()
        {
            action_out = Some(ReviewAction::DeleteFeedback(feedback_id));
        }
    });

    action_out
}

fn status_options(theme: &Theme) -> [PopupOption<ReviewStatus>; 4] {
    [
        ReviewStatus::Todo,
        ReviewStatus::InProgress,
        ReviewStatus::Done,
        ReviewStatus::Ignored,
    ]
    .map(|status| {
        let v = crate::ui::views::review::visuals::status_visuals(status, theme);
        PopupOption {
            label: v.label,
            value: status,
            fg: v.color,
            icon: Some(v.icon),
        }
    })
}

fn impact_options(theme: &Theme) -> [PopupOption<FeedbackImpact>; 3] {
    [
        FeedbackImpact::Blocking,
        FeedbackImpact::NiceToHave,
        FeedbackImpact::Nitpick,
    ]
    .map(|impact| {
        let v = crate::ui::views::review::visuals::impact_visuals(impact, theme);
        PopupOption {
            label: v.label,
            value: impact,
            fg: v.color,
            icon: Some(v.icon),
        }
    })
}
