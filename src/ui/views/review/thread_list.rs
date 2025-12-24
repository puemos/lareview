use crate::domain::{ReviewStatus, Thread};
use crate::ui::app::ReviewAction;
use crate::ui::components::list_item::ListItem;
use crate::ui::theme::Theme;
use crate::ui::{icons, spacing, typography};
use eframe::egui;

pub fn render_thread_list(
    ui: &mut egui::Ui,
    threads: &[Thread],
    active_thread_id: Option<&str>,
    theme: &Theme,
) -> Option<ReviewAction> {
    let mut action = None;

    if threads.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(spacing::SPACING_XL);
            ui.label(
                typography::body(icons::ICON_EMPTY)
                    .size(24.0)
                    .color(theme.text_muted),
            );
            ui.add_space(spacing::SPACING_XS);
            ui.label(
                typography::body("No threads yet")
                    .size(14.0)
                    .color(theme.text_muted),
            );
        });
        return None;
    }

    // Sort threads: Open/WIP first, then by date (newest first)
    let mut display_threads: Vec<&Thread> = threads.iter().collect();
    display_threads.sort_by(|a, b| {
        let rank_a = a.status.rank();
        let rank_b = b.status.rank();
        if rank_a != rank_b {
            rank_a.cmp(&rank_b)
        } else {
            b.updated_at.cmp(&a.updated_at)
        }
    });

    egui::ScrollArea::vertical()
        .id_salt("thread_list_scroll")
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            for thread in display_threads {
                let is_active = active_thread_id.is_some_and(|id| id == thread.id);

                // -- Status Icon --
                let (icon, color) = match thread.status {
                    ReviewStatus::Todo => (icons::STATUS_TODO, theme.text_muted),
                    ReviewStatus::InProgress => (icons::STATUS_WIP, theme.accent),
                    ReviewStatus::Done => (icons::STATUS_DONE, theme.success),
                    ReviewStatus::Ignored => (icons::STATUS_IGNORED, theme.destructive),
                };

                // -- Title --
                let title_text = typography::bold(&thread.title).color(theme.text_primary);

                // -- Metadata (Impact + Time) --
                let (impact_icon, impact_label, impact_color) = match thread.impact {
                    crate::domain::ThreadImpact::Blocking => {
                        (icons::IMPACT_BLOCKING, "Blocking", theme.destructive)
                    }
                    crate::domain::ThreadImpact::NiceToHave => {
                        (icons::IMPACT_NICE_TO_HAVE, "Nice-to-have", theme.warning)
                    }
                    crate::domain::ThreadImpact::Nitpick => {
                        (icons::IMPACT_NITPICK, "Nitpick", theme.text_muted)
                    }
                };

                let time_str = super::format_timestamp(&thread.updated_at);

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
                let response = ListItem::new(title_text)
                    .status_icon(icon, color)
                    .metadata(metadata)
                    .selected(is_active)
                    .action(|| {
                        action = Some(ReviewAction::NavigateToThread(thread.clone()));
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
