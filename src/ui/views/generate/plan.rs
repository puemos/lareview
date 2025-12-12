use agent_client_protocol::{Plan, PlanEntryStatus};
use catppuccin_egui::MOCHA;
use eframe::egui;

pub(super) fn render_plan_panel(ui: &mut egui::Ui, plan: &Plan) {
    if plan.entries.is_empty() {
        return;
    }

    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("{} PLAN", egui_phosphor::regular::LIST_CHECKS))
                        .size(11.0)
                        .color(MOCHA.subtext0),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let total = plan.entries.len();
                    let completed = plan
                        .entries
                        .iter()
                        .filter(|e| matches!(&e.status, PlanEntryStatus::Completed))
                        .count();
                    ui.label(
                        egui::RichText::new(format!(
                            "{} {completed}/{total}",
                            egui_phosphor::regular::CHECK_CIRCLE
                        ))
                        .size(11.0)
                        .color(MOCHA.overlay2),
                    );
                });
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);

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
            .color(MOCHA.subtext0)
            .size(12.0),
        );
        return;
    }

    let total = plan.entries.len();
    let completed = plan
        .entries
        .iter()
        .filter(|e| matches!(&e.status, PlanEntryStatus::Completed))
        .count();

    let default_open = plan.entries.iter().any(|e| {
        matches!(
            &e.status,
            PlanEntryStatus::InProgress | PlanEntryStatus::Pending
        )
    });

    let header = egui::RichText::new(format!(
        "{} Plan ({completed}/{total})",
        egui_phosphor::regular::LIST_CHECKS
    ))
    .color(MOCHA.subtext0)
    .size(12.0);

    egui::CollapsingHeader::new(header)
        .id_salt(("plan", "timeline"))
        .default_open(default_open)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            render_plan_entries(ui, plan, /*dense=*/ true);
        });
}

fn render_plan_entries(ui: &mut egui::Ui, plan: &Plan, dense: bool) {
    for (idx, entry) in plan.entries.iter().enumerate() {
        let status = entry.status.clone();
        let (icon, color, label) = plan_entry_style(status.clone());

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

            ui.label(egui::RichText::new(icon).size(14.0).color(color));

            let text_color = match status {
                PlanEntryStatus::Completed => MOCHA.subtext1,
                PlanEntryStatus::InProgress => MOCHA.text,
                PlanEntryStatus::Pending => MOCHA.text,
                _ => MOCHA.text,
            };

            ui.add(
                egui::Label::new(
                    egui::RichText::new(&entry.content)
                        .color(text_color)
                        .size(if dense { 12.0 } else { 12.5 }),
                )
                .wrap(),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(label).size(10.5).color(MOCHA.overlay2));
            });
        });

        if !dense && idx + 1 < plan.entries.len() {
            ui.add_space(2.0);
        }
    }
}

fn plan_entry_style(status: PlanEntryStatus) -> (&'static str, egui::Color32, &'static str) {
    match status {
        PlanEntryStatus::Completed => (egui_phosphor::regular::CHECK_CIRCLE, MOCHA.green, "done"),
        PlanEntryStatus::InProgress => {
            (egui_phosphor::regular::CIRCLE_DASHED, MOCHA.yellow, "doing")
        }
        PlanEntryStatus::Pending => (egui_phosphor::regular::CIRCLE, MOCHA.overlay1, "todo"),
        _ => (egui_phosphor::regular::CIRCLE, MOCHA.overlay1, "unknown"),
    }
}
