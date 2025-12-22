use crate::domain::{Plan, PlanStatus};
use crate::ui::theme::current_theme;
use eframe::egui;

use crate::ui::spacing;

pub(super) fn render_plan_panel(ui: &mut egui::Ui, plan: &Plan) {
    if plan.entries.is_empty() {
        return;
    }

    egui::Frame::group(ui.style())
        .fill(current_theme().bg_secondary)
        .stroke(egui::Stroke::new(1.0, current_theme().border))
        .corner_radius(egui::CornerRadius::same(spacing::RADIUS_MD))
        .inner_margin(egui::Margin::symmetric(
            spacing::SPACING_MD as i8,
            spacing::SPACING_SM as i8,
        )) // 10,8 -> 12,8
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("{} PLAN", egui_phosphor::regular::LIST_CHECKS))
                        .size(11.0)
                        .color(current_theme().text_muted),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let total = plan.entries.len();
                    let completed = plan
                        .entries
                        .iter()
                        .filter(|e| matches!(&e.status, PlanStatus::Completed))
                        .count();
                    ui.label(
                        egui::RichText::new(format!(
                            "{} {completed}/{total}",
                            egui_phosphor::regular::CHECK_CIRCLE
                        ))
                        .size(11.0)
                        .color(current_theme().text_muted),
                    );
                });
            });

            ui.add_space(spacing::SPACING_XS + 2.0);
            ui.separator();
            ui.add_space(spacing::SPACING_XS);

            render_plan_entries(ui, plan, /*dense=*/ false);
        });
}

pub(super) fn render_plan_timeline_item(ui: &mut egui::Ui, plan: &Plan) {
    if plan.entries.is_empty() {
        ui.label(
            egui::RichText::new(format!(
                "{} Plan updated",
                egui_phosphor::regular::LIST_CHECKS
            ))
            .color(current_theme().text_muted)
            .size(12.0),
        );
        return;
    }

    let total = plan.entries.len();
    let completed = plan
        .entries
        .iter()
        .filter(|e| matches!(&e.status, PlanStatus::Completed))
        .count();

    let default_open = plan
        .entries
        .iter()
        .any(|e| matches!(&e.status, PlanStatus::InProgress | PlanStatus::Pending));

    let header = egui::RichText::new(format!(
        "{} Plan ({completed}/{total})",
        egui_phosphor::regular::LIST_CHECKS
    ))
    .color(current_theme().text_muted)
    .size(12.0);

    egui::CollapsingHeader::new(header)
        .id_salt(("plan", "timeline"))
        .default_open(default_open)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing =
                egui::vec2(spacing::SPACING_XS + 2.0, spacing::SPACING_XS + 2.0);
            render_plan_entries(ui, plan, /*dense=*/ true);
        });
}

fn render_plan_entries(ui: &mut egui::Ui, plan: &Plan, dense: bool) {
    for (idx, entry) in plan.entries.iter().enumerate() {
        let status = entry.status;
        let (icon, color, _label) = plan_entry_style(status);

        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(spacing::SPACING_SM, 0.0); // 8.0, 0.0

            ui.label(egui::RichText::new(icon).size(14.0).color(color));

            let text_color = match status {
                PlanStatus::Completed => current_theme().text_muted,
                PlanStatus::InProgress => current_theme().text_primary,
                PlanStatus::Pending => current_theme().text_primary,
            };

            ui.add(
                egui::Label::new(
                    egui::RichText::new(&entry.content)
                        .monospace()
                        .color(text_color)
                        .size(if dense { 12.0 } else { 12.5 }),
                )
                .wrap(),
            );
        });

        if !dense && idx + 1 < plan.entries.len() {
            ui.add_space(2.0); // Keep 2.0 as this is specific spacing for plan entry gaps
        }
    }
}

fn plan_entry_style(status: PlanStatus) -> (&'static str, egui::Color32, &'static str) {
    match status {
        PlanStatus::Completed => (
            egui_phosphor::regular::CHECK_CIRCLE,
            current_theme().success,
            "done",
        ),
        PlanStatus::InProgress => (
            egui_phosphor::regular::CIRCLE_DASHED,
            current_theme().warning,
            "doing",
        ),
        PlanStatus::Pending => (
            egui_phosphor::regular::CIRCLE,
            current_theme().text_muted,
            "todo",
        ),
    }
}
