use crate::domain::ReviewTask;
use crate::ui::app::ReviewAction;
use crate::ui::{icons, spacing, typography};
use crate::ui::theme::Theme;
use eframe::egui;

pub(crate) fn render_task_header(
    ui: &mut egui::Ui,
    task: &ReviewTask,
    theme: &Theme,
) -> Option<ReviewAction> {
    let mut status_action = None;

    // 1. Task Title
    ui.add(
        egui::Label::new(
            typography::bold(&task.title)
                .size(22.0)
                .line_height(Some(32.0))
                .color(theme.text_primary),
        )
        .wrap(),
    );

    ui.add_space(spacing::SPACING_SM);

    // 2. Metadata row (Status + Risk + Stats)
    let row_height = 28.0;
    let status_width = 140.0;

    let status_visuals = |status: crate::domain::ReviewStatus| match status {
        crate::domain::ReviewStatus::Todo => (icons::STATUS_TODO, "To Do", theme.text_muted),
        crate::domain::ReviewStatus::InProgress => (icons::STATUS_WIP, "In Progress", theme.accent),
        crate::domain::ReviewStatus::Done => (icons::STATUS_DONE, "Done", theme.success),
        crate::domain::ReviewStatus::Ignored => {
            (icons::STATUS_IGNORED, "Ignored", theme.destructive)
        }
    };

    let status_widget_text =
        |icon: &str, icon_color: egui::Color32, label: &str, label_color: egui::Color32| {
            let mut job = egui::text::LayoutJob::default();
            let icon_format = egui::text::TextFormat {
                font_id: egui::FontId::proportional(12.0),
                color: icon_color,
                ..Default::default()
            };
            let label_format = egui::text::TextFormat {
                font_id: egui::FontId::proportional(12.0),
                color: label_color,
                ..Default::default()
            };
            job.append(icon, 0.0, icon_format);
            job.append(label, 6.0, label_format);
            egui::WidgetText::from(job)
        };

    ui.scope(|ui| {
        let old_interact_size = ui.spacing().interact_size;
        ui.spacing_mut().interact_size.y = row_height;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = spacing::SPACING_SM;

            // Status Dropdown
            let (selected_icon, selected_label, selected_color) = status_visuals(task.status);
            let selected_text = status_widget_text(
                selected_icon,
                selected_color,
                selected_label,
                theme.text_primary,
            );

            egui::ComboBox::from_id_salt(ui.id().with(("task_status", &task.id)))
                .selected_text(selected_text)
                .width(status_width)
                .show_ui(ui, |ui| {
                    for status in [
                        crate::domain::ReviewStatus::Todo,
                        crate::domain::ReviewStatus::InProgress,
                        crate::domain::ReviewStatus::Done,
                        crate::domain::ReviewStatus::Ignored,
                    ] {
                        let (icon, label, color) = status_visuals(status);
                        let text = status_widget_text(icon, color, label, theme.text_primary);
                        let selected = task.status == status;
                        if ui.selectable_label(selected, text).clicked() {
                            status_action = Some(ReviewAction::UpdateTaskStatus {
                                task_id: task.id.clone(),
                                status,
                            });
                        }
                    }
                });

            // Dot Separator
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

            ui.label(
                typography::body("|")
                    .color(theme.text_disabled)
                    .size(12.0),
            );

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

            ui.label(
                typography::body("lines")
                    .color(theme.text_muted)
                    .size(12.0),
            );
        });

        ui.spacing_mut().interact_size = old_interact_size;
    });

    status_action
}
