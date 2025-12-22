use crate::domain::ReviewTask;
use crate::ui::app::ReviewAction;
use crate::ui::spacing;
use crate::ui::theme::Theme;
use eframe::egui;
use egui_phosphor::regular as icons;

pub(crate) fn render_task_header(
    ui: &mut egui::Ui,
    task: &ReviewTask,
    theme: &Theme,
) -> Option<ReviewAction> {
    let mut status_action = None;

    // 1. Task Title
    ui.add(
        egui::Label::new(
            egui::RichText::new(&task.title)
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

    let status_visuals = |status: crate::domain::TaskStatus| match status {
        crate::domain::TaskStatus::Pending => (icons::CIRCLE, "To do", theme.brand),
        crate::domain::TaskStatus::InProgress => (icons::CIRCLE_HALF, "In progress", theme.accent),
        crate::domain::TaskStatus::Done => (icons::CHECK_CIRCLE, "Done", theme.success),
        crate::domain::TaskStatus::Ignored => (icons::X_CIRCLE, "Ignored", theme.destructive),
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
                        crate::domain::TaskStatus::Pending,
                        crate::domain::TaskStatus::InProgress,
                        crate::domain::TaskStatus::Done,
                        crate::domain::TaskStatus::Ignored,
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
            ui.label(egui::RichText::new("·").color(theme.text_muted).size(14.0));
            ui.add_space(spacing::SPACING_XS);

            // Risk Indicator
            let (risk_icon, risk_fg, risk_label) = match task.stats.risk {
                crate::domain::RiskLevel::High => (
                    icons::CARET_CIRCLE_DOUBLE_UP,
                    theme.destructive,
                    "High risk",
                ),
                crate::domain::RiskLevel::Medium => {
                    (icons::CARET_CIRCLE_UP, theme.warning, "Med risk")
                }
                crate::domain::RiskLevel::Low => {
                    (icons::CARET_CIRCLE_DOWN, theme.accent, "Low risk")
                }
            };

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label(egui::RichText::new(risk_icon).color(risk_fg).size(14.0));
                ui.label(
                    egui::RichText::new(risk_label)
                        .color(theme.text_muted)
                        .size(12.0),
                );
            });

            // Dot Separator
            ui.add_space(spacing::SPACING_XS);
            ui.label(egui::RichText::new("·").color(theme.text_muted).size(14.0));
            ui.add_space(spacing::SPACING_XS);

            // Stats
            ui.label(
                egui::RichText::new(format!("{} files", task.files.len()))
                    .color(theme.text_muted)
                    .size(12.0),
            );

            ui.label(
                egui::RichText::new("|")
                    .color(theme.text_disabled)
                    .size(12.0),
            );

            ui.label(
                egui::RichText::new(format!("+{}", task.stats.additions))
                    .color(theme.success)
                    .size(12.0),
            );

            ui.label(
                egui::RichText::new(format!("-{}", task.stats.deletions))
                    .color(theme.destructive)
                    .size(12.0),
            );

            ui.label(
                egui::RichText::new("lines")
                    .color(theme.text_muted)
                    .size(12.0),
            );
        });

        ui.spacing_mut().interact_size = old_interact_size;
    });

    status_action
}
