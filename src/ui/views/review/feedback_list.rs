use crate::domain::{Feedback, ReviewStatus};
use crate::ui::app::ReviewAction;
use crate::ui::components::list_item::ListItem;
use crate::ui::theme::Theme;
use crate::ui::{icons, spacing, typography};
use eframe::egui;

pub fn render_feedback_list(
    ui: &mut egui::Ui,
    feedbacks: &[Feedback],
    active_feedback_id: Option<&str>,
    select_mode: bool,
    selected_ids: &std::collections::HashSet<String>,
    show_status_icons: bool,
    theme: &Theme,
) -> Option<ReviewAction> {
    let mut action = None;

    if feedbacks.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(spacing::SPACING_XL);
            ui.label(
                typography::body(icons::ICON_EMPTY)
                    .size(24.0)
                    .color(theme.text_muted),
            );
            ui.add_space(spacing::SPACING_XS);
            ui.label(
                typography::body("No feedback yet")
                    .size(14.0)
                    .color(theme.text_muted),
            );
        });
        return None;
    }

    // Sort feedbacks: Open/WIP first, then by date (newest first)
    let mut display_feedbacks: Vec<&Feedback> = feedbacks.iter().collect();
    display_feedbacks.sort_by(|a, b| {
        let rank_a = a.status.rank();
        let rank_b = b.status.rank();
        if rank_a != rank_b {
            rank_a.cmp(&rank_b)
        } else {
            b.updated_at.cmp(&a.updated_at)
        }
    });

    egui::ScrollArea::vertical()
        .id_salt("feedback_list_scroll")
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            for feedback in display_feedbacks {
                let is_active = active_feedback_id.is_some_and(|id| id == feedback.id);

                // -- Status Icon --
                let (icon, color) = match feedback.status {
                    ReviewStatus::Todo => (icons::STATUS_TODO, theme.text_muted),
                    ReviewStatus::InProgress => (icons::STATUS_WIP, theme.accent),
                    ReviewStatus::Done => (icons::STATUS_DONE, theme.success),
                    ReviewStatus::Ignored => (icons::STATUS_IGNORED, theme.destructive),
                };

                // -- Title --
                let title_text = typography::bold(&feedback.title).color(theme.text_primary);

                // -- Metadata (Impact + Time) --
                let (impact_icon, impact_label, impact_color) = match feedback.impact {
                    crate::domain::FeedbackImpact::Blocking => {
                        (icons::IMPACT_BLOCKING, "Blocking", theme.destructive)
                    }
                    crate::domain::FeedbackImpact::NiceToHave => {
                        (icons::IMPACT_NICE_TO_HAVE, "Nice-to-have", theme.warning)
                    }
                    crate::domain::FeedbackImpact::Nitpick => {
                        (icons::IMPACT_NITPICK, "Nitpick", theme.text_muted)
                    }
                };

                let time_str = super::format_timestamp(&feedback.updated_at);

                // Create a job for the metadata with icon + label + time
                let mut metadata_job = egui::text::LayoutJob::default();

                // Add impact icon
                metadata_job.append(
                    impact_icon,
                    0.0,
                    egui::TextFormat {
                        font_id: egui::FontId::proportional(10.0),
                        color: impact_color,
                        ..Default::default()
                    },
                );

                // Add impact label
                metadata_job.append(
                    &format!(" {} ", impact_label),
                    0.0,
                    egui::TextFormat {
                        font_id: egui::FontId::proportional(10.0),
                        color: impact_color,
                        ..Default::default()
                    },
                );

                // Add separator
                metadata_job.append(
                    "· ",
                    0.0,
                    egui::TextFormat {
                        font_id: egui::FontId::proportional(10.0),
                        color: theme.text_disabled,
                        ..Default::default()
                    },
                );

                // Add time
                metadata_job.append(
                    &time_str,
                    0.0,
                    egui::TextFormat {
                        font_id: egui::FontId::proportional(10.0),
                        color: theme.text_muted,
                        ..Default::default()
                    },
                );

                let metadata = egui::WidgetText::from(metadata_job);

                // -- Render Item --
                let mut list_item = ListItem::new(title_text)
                    .metadata(metadata)
                    .inner_margin(egui::Margin::same(spacing::SPACING_MD as i8))
                    .selected(is_active);

                if show_status_icons {
                    list_item = list_item.status_icon(icon, color);
                }

                if select_mode {
                    let mut selected = selected_ids.contains(&feedback.id);
                    list_item = list_item.checkbox(&mut selected);
                }

                let response = list_item
                    .action(|| {
                        if select_mode {
                            action =
                                Some(ReviewAction::ToggleFeedbackSelection(feedback.id.clone()));
                        } else {
                            action = Some(ReviewAction::NavigateToFeedback(feedback.clone()));
                        }
                    })
                    .show_with_bg(ui, theme);

                // Separator
                ui.painter().line_segment(
                    [
                        // add 6.5 to make the line touch the left edge of the resize ha
                        egui::pos2(response.rect.min.x - 6.5, response.rect.max.y),
                        egui::pos2(response.rect.max.x, response.rect.max.y),
                    ],
                    egui::Stroke::new(1.0, theme.border),
                );
            }
        });

    action
}
