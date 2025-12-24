use crate::domain::{LinkedRepo, Plan};
use crate::ui::app::{GenerateAction, SelectedAgent};
use crate::ui::spacing;
use crate::ui::theme::Theme;
use eframe::egui;

pub(crate) struct AgentPaneContext<'a> {
    pub selected_agent: &'a SelectedAgent,
    pub selected_repo_id: Option<&'a String>,
    pub linked_repos: &'a [LinkedRepo],
    pub latest_plan: Option<&'a Plan>,
}

pub(crate) fn render_agent_pane(
    ui: &mut egui::Ui,
    ctx: AgentPaneContext<'_>,
    _theme: &Theme,
) -> Option<GenerateAction> {
    let mut action_out = None;

    egui::Frame::NONE
        .inner_margin(spacing::SPACING_SM)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing =
                egui::vec2(spacing::BUTTON_PADDING.0, spacing::BUTTON_PADDING.1);

            ui.add_space(spacing::SPACING_XS);

            // Integrated Control Panel
            ui.vertical(|ui| {
                // 1. Configuration Row
                ui.horizontal(|ui| {
                    let mut temp_agent = ctx.selected_agent.clone();
                    crate::ui::components::agent_selector::agent_selector(ui, &mut temp_agent);
                    if temp_agent != *ctx.selected_agent {
                        action_out = Some(GenerateAction::SelectAgent(temp_agent));
                    }

                    ui.add_space(spacing::SPACING_SM);

                    let mut temp_repo_id = ctx.selected_repo_id.cloned();
                    crate::ui::components::repo_selector::repo_selector(
                        ui,
                        &mut temp_repo_id,
                        ctx.linked_repos,
                    );
                    if temp_repo_id.as_ref() != ctx.selected_repo_id {
                        action_out = Some(GenerateAction::SelectRepo(temp_repo_id));
                    }
                });
            });

            ui.add_space(spacing::SPACING_SM);

            // Plan Section
            if let Some(plan) = ctx.latest_plan {
                ui.add_space(spacing::SPACING_SM);
                crate::ui::views::generate::plan::render_plan_panel(ui, plan);
            }
        });

    action_out
}
