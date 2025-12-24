use crate::application::review::ordering::{
    sub_flows_in_display_order, tasks_in_sub_flow_display_order,
};
use crate::domain::ReviewTask;
use crate::ui::app::ReviewAction;
use crate::ui::components::list_item::ListItem;
use crate::ui::theme::Theme;
use crate::ui::{icons, spacing, typography};
use eframe::egui;

/// Renders the logic for the Left Panel (Navigation)
pub(crate) fn render_navigation_tree(
    ui: &mut egui::Ui,
    tasks_by_sub_flow: &std::collections::HashMap<Option<String>, Vec<ReviewTask>>,
    selected_task_id: Option<&String>,
    is_generating: bool,
    theme: &Theme,
) -> Option<ReviewAction> {
    let sub_flows = sub_flows_in_display_order(tasks_by_sub_flow);

    egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(spacing::SPACING_SM as i8, 0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    egui::Label::new(typography::bold("Tasks").color(theme.text_primary)).wrap(),
                );
            });
        });

    if sub_flows.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(
                typography::body("No tasks loaded")
                    .italics()
                    .color(theme.text_muted),
            );
        });
        return None;
    }

    ui.add_space(spacing::SPACING_SM);

    ui.spacing_mut().item_spacing = egui::vec2(0.0, spacing::SPACING_SM);
    ui.spacing_mut().indent = 12.0;
    ui.visuals_mut().indent_has_left_vline = true;
    ui.visuals_mut().widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, theme.border);

    let mut action_out = None;

    for (sub_flow_name, tasks) in sub_flows {
        let title = sub_flow_name.as_deref().unwrap_or("UNCATEGORIZED");
        let title_upper = title.to_uppercase();
        let total = tasks.len();
        let finished = tasks.iter().filter(|t| t.status.is_closed()).count();
        let is_done = finished == total && total > 0;

        let header_id = ui.id().with(("sub_flow_collapse", title));

        ui.set_width(ui.available_width());

        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), header_id, true)
            .show_header(ui, |ui| {
                ui.horizontal(|ui| {
                    let mut heading = typography::bold(&title_upper)
                        .size(11.0)
                        .extra_letter_spacing(0.5);

                    if is_done {
                        heading = heading.color(theme.text_muted);
                    } else {
                        heading = heading.color(theme.text_primary);
                    }

                    ui.label(heading);

                    ui.add_space(spacing::SPACING_XS);

                    let color = if is_done {
                        theme.success
                    } else {
                        theme.text_muted
                    };

                    let count_text = typography::body(format!("{}/{}", finished, total))
                        .size(11.0)
                        .color(color);

                    ui.label(count_text);
                });
            })
            .body(|ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing = egui::vec2(0.0, spacing::SPACING_XS);
                for task in tasks_in_sub_flow_display_order(tasks) {
                    if let Some(action) = render_nav_item(ui, task, selected_task_id, theme) {
                        action_out = Some(action);
                    }
                }
            });
    }

    action_out
}

/// Renders a single task item in the sidebar
pub(crate) fn render_nav_item(
    ui: &mut egui::Ui,
    task: &ReviewTask,
    selected_task_id: Option<&String>,
    theme: &Theme,
) -> Option<ReviewAction> {
    let is_selected = selected_task_id == Some(&task.id);

    // -- Status Icon --
    let (status_icon, status_color) = match task.status {
        crate::domain::ReviewStatus::Todo => (icons::STATUS_TODO, theme.text_muted),
        crate::domain::ReviewStatus::InProgress => (icons::STATUS_WIP, theme.accent),
        crate::domain::ReviewStatus::Done => (icons::STATUS_DONE, theme.success),
        crate::domain::ReviewStatus::Ignored => (icons::STATUS_IGNORED, theme.destructive),
    };

    // -- Title --
    let mut title_text = typography::body(&task.title)
        .size(13.0)
        .color(if is_selected {
            theme.text_primary
        } else {
            theme.text_secondary
        });

    if task.status.is_closed() {
        title_text = title_text.strikethrough().color(theme.text_muted);
    }

    // -- Risk / Subtitle --
    let (risk_icon, risk_color, risk_label) = match task.stats.risk {
        crate::domain::RiskLevel::High => (icons::RISK_HIGH, theme.destructive, "High Risk"),
        crate::domain::RiskLevel::Medium => (icons::RISK_MEDIUM, theme.warning, "Medium Risk"),
        crate::domain::RiskLevel::Low => (icons::RISK_LOW, theme.accent, "Low Risk"),
    };

    let subtitle = typography::body(format!("{}  {}", risk_icon, risk_label))
        .size(11.0)
        .color(risk_color);

    let mut action_out = None;

    ListItem::new(title_text)
        .status_icon(status_icon, status_color)
        .subtitle(subtitle)
        .selected(is_selected)
        .action(|| {
            action_out = Some(ReviewAction::SelectTask {
                task_id: task.id.clone(),
            });
        })
        .show_with_bg(ui, theme);

    action_out
}
