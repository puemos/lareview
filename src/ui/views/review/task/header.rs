use crate::domain::{ReviewStatus, ReviewTask};
use crate::ui::app::ReviewAction;
use crate::ui::components::{PopupOption, popup_selector};
use crate::ui::theme::Theme;
use crate::ui::{icons, spacing, typography};
use eframe::egui;

pub(crate) fn render_task_header(
    ui: &mut egui::Ui,
    task: &ReviewTask,
    theme: &Theme,
) -> Option<ReviewAction> {
    let mut status_action = None;

    // 1. Task Title
// ...
    // 2. Metadata row (Status + Risk + Stats)
    let row_height = 28.0;
    let status_width = 140.0;

    ui.scope(|ui| {
        let old_interact_size = ui.spacing().interact_size;
        ui.spacing_mut().interact_size.y = row_height;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = spacing::SPACING_SM;

            // Status Dropdown
            let status_choices = [
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
            });

            if let Some(next_status) = popup_selector(
                ui,
                ui.id().with(("task_status_popup", &task.id)),
                task.status,
                &status_choices,
                status_width,
                true, // enabled
            ) {
                status_action = Some(ReviewAction::UpdateTaskStatus {
                    task_id: task.id.clone(),
                    status: next_status,
                });
            }

            // Dot Separator
// ...
            ui.add_space(spacing::SPACING_XS);
            ui.label(typography::body("·").color(theme.text_muted).size(14.0));
            ui.add_space(spacing::SPACING_XS);

            // Risk Indicator
            let (risk_icon, risk_fg, risk_label) = match task.stats.risk {
                crate::domain::RiskLevel::High => {
                    (icons::RISK_HIGH, theme.destructive, "High risk")
                }
                crate::domain::RiskLevel::Medium => (icons::RISK_MEDIUM, theme.warning, "Med risk"),
                crate::domain::RiskLevel::Low => (icons::RISK_LOW, theme.accent, "Low risk"),
            };

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label(typography::body(risk_icon).color(risk_fg).size(14.0));
                ui.label(
                    typography::body(risk_label)
                        .color(theme.text_muted)
                        .size(12.0),
                );
            });

            // Dot Separator
            ui.add_space(spacing::SPACING_XS);
            ui.label(typography::body("·").color(theme.text_muted).size(14.0));
            ui.add_space(spacing::SPACING_XS);

            // Stats
            ui.label(
                typography::body(format!("{} files", task.files.len()))
                    .color(theme.text_muted)
                    .size(12.0),
            );

            ui.label(typography::body("|").color(theme.text_disabled).size(12.0));

            ui.label(
                typography::body(format!("+{}", task.stats.additions))
                    .color(theme.success)
                    .size(12.0),
            );

            ui.label(
                typography::body(format!("-{}", task.stats.deletions))
                    .color(theme.destructive)
                    .size(12.0),
            );

            ui.label(typography::body("lines").color(theme.text_muted).size(12.0));
        });

        ui.spacing_mut().interact_size = old_interact_size;
    });

    status_action
}
