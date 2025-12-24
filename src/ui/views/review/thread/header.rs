use crate::domain::{ReviewStatus, Thread, ThreadImpact};
use crate::ui::app::ReviewAction;
use crate::ui::components::{PopupOption, popup_selector};
use crate::ui::theme::Theme;
use crate::ui::{spacing, typography};
use eframe::egui;
use egui::Color32;

pub(crate) fn render_thread_header(
    ui: &mut egui::Ui,
    thread: Option<&Thread>,
    thread_title_draft: &str,
    theme: &Theme,
    task_id: &str,
) -> Option<ReviewAction> {
    let mut action_out = None;

    let thread_id = thread.map(|t| t.id.clone());
    let existing_title = thread.map(|t| t.title.clone()).unwrap_or_default();
    let can_edit_thread = thread_id.is_some();

    ui.horizontal(|ui| {
        let status_width = 120.0;
        let impact_width = 150.0;
        let selector_gap = spacing::SPACING_MD;
        let selector_total_width = status_width + impact_width + selector_gap;
        let title_width = (ui.available_width() - selector_total_width).max(120.0);

        // Edit Title
        let mut edit_text = thread_title_draft.to_string();

        let response = ui
            .scope(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut edit_text)
                        .hint_text("Discussion Title")
                        .desired_width(title_width)
                        .text_color(Color32::WHITE)
                        .text_color_opt(Some(theme.text_muted))
                        .font(typography::body_font(16.0))
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                )
            })
            .inner;

        if response.changed() {
            action_out = Some(ReviewAction::SetThreadTitleDraft {
                text: edit_text.clone(),
            });
        }

        if response.lost_focus()
            && edit_text != existing_title
            && let Some(thread_id) = thread_id.clone()
        {
            action_out = Some(ReviewAction::UpdateThreadTitle {
                thread_id,
                title: edit_text.clone(),
            });
        }

        // Disable automatic item spacing for precise control
        let old_spacing = ui.spacing().item_spacing.x;
        ui.spacing_mut().item_spacing.x = 0.0;

        let status_choices = status_options(theme);
        let impact_choices = impact_options(theme);

        // Status selector
        let status = thread.map(|t| t.status).unwrap_or(ReviewStatus::Todo);
        if let Some(next_status) = popup_selector(
            ui,
            ui.make_persistent_id(("thread_status_popup", task_id, &thread_id)),
            status,
            &status_choices,
            status_width,
            can_edit_thread,
        ) && let Some(thread_id) = thread_id.clone()
        {
            action_out = Some(ReviewAction::UpdateThreadStatus {
                thread_id,
                status: next_status,
            });
        }

        ui.add_space(selector_gap);

        // Impact selector
        let impact = thread.map(|t| t.impact).unwrap_or(ThreadImpact::Nitpick);
        if let Some(next_impact) = popup_selector(
            ui,
            ui.make_persistent_id(("thread_impact_popup", task_id, &thread_id)),
            impact,
            &impact_choices,
            impact_width,
            can_edit_thread,
        ) && let Some(thread_id) = thread_id.clone()
        {
            action_out = Some(ReviewAction::UpdateThreadImpact {
                thread_id,
                impact: next_impact,
            });
        }

        ui.spacing_mut().item_spacing.x = old_spacing;
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

fn impact_options(theme: &Theme) -> [PopupOption<ThreadImpact>; 3] {
    [
        ThreadImpact::Blocking,
        ThreadImpact::NiceToHave,
        ThreadImpact::Nitpick,
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
