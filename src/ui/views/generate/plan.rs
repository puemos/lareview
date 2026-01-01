use crate::domain::{Plan, PlanStatus};
use crate::ui::theme::current_theme;
use crate::ui::{icons, typography};
use eframe::egui;

use crate::ui::components::rotating_icon::rotating_icon;
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
        ))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    typography::body(format!("{} PLAN", icons::ICON_PLAN))
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
                        typography::body(format!("{} {completed}/{total}", icons::STATUS_DONE))
                            .size(11.0)
                            .color(current_theme().text_muted),
                    );
                });
            });

            ui.add_space(spacing::SPACING_XS + 2.0);
            ui.separator();
            ui.add_space(spacing::SPACING_XS);

            render_plan_entries(ui, plan, false);
        });
}

pub(super) fn render_plan_timeline_item(ui: &mut egui::Ui, plan: &Plan) {
    if plan.entries.is_empty() {
        ui.label(
            typography::body(format!("{} Plan updated", icons::ICON_PLAN))
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

    let header = typography::body(format!("{} Plan ({completed}/{total})", icons::ICON_PLAN))
        .color(current_theme().text_muted)
        .size(12.0);

    egui::CollapsingHeader::new(header)
        .id_salt(("plan", "timeline"))
        .default_open(default_open)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing =
                egui::vec2(spacing::SPACING_XS + 2.0, spacing::SPACING_XS + 2.0);
            render_plan_entries(ui, plan, true);
        });
}

fn render_plan_entries(ui: &mut egui::Ui, plan: &Plan, dense: bool) {
    for (idx, entry) in plan.entries.iter().enumerate() {
        let status = entry.status;
        let (icon, color, _label) = plan_entry_style(status);

        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(spacing::SPACING_SM, 0.0);

            if status == PlanStatus::InProgress {
                rotating_icon(ui, icon, color, 14.0);
            } else {
                ui.label(typography::body(icon).size(14.0).color(color));
            }

            let text_color = match status {
                PlanStatus::Completed => current_theme().text_muted,
                PlanStatus::InProgress => current_theme().text_primary,
                PlanStatus::Pending => current_theme().text_primary,
            };

            ui.add(
                egui::Label::new(
                    typography::small_mono(&entry.content)
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
        PlanStatus::Completed => (icons::STATUS_DONE, current_theme().status_done, "done"),
        PlanStatus::InProgress => (
            icons::STATUS_IN_PROGRESS,
            current_theme().status_in_progress,
            "doing",
        ),
        PlanStatus::Pending => (
            icons::STATUS_IN_PROGRESS,
            current_theme().status_todo,
            "todo",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::PlanEntry;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    #[test]
    fn test_render_plan_panel() {
        let plan = Plan {
            entries: vec![
                PlanEntry {
                    content: "Step 1".to_string(),
                    status: PlanStatus::Completed,
                    priority: crate::domain::PlanPriority::Medium,
                    meta: None,
                },
                PlanEntry {
                    content: "Step 2".to_string(),
                    status: PlanStatus::InProgress,
                    priority: crate::domain::PlanPriority::Medium,
                    meta: None,
                },
            ],
            meta: None,
        };

        let mut harness = Harness::new_ui(|ui| {
            render_plan_panel(ui, &plan);
        });
        harness.run_steps(2);

        harness
            .get_all_by_role(egui::accesskit::Role::Label)
            .into_iter()
            .find(|n| format!("{:?}", n).contains("PLAN"))
            .expect("PLAN label not found");
        harness.get_by_label("Step 1");
        harness.get_by_label("Step 2");
        harness
            .get_all_by_role(egui::accesskit::Role::Label)
            .into_iter()
            .find(|n| format!("{:?}", n).contains("1/2"))
            .expect("Counter label not found");
    }
}
