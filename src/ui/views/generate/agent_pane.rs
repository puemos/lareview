use crate::domain::{LinkedRepo, Plan};
use crate::ui::app::{GenerateAction, SelectedAgent};
use crate::ui::components::cyber_button::cyber_button;
use crate::ui::components::status::error_banner;
use crate::ui::spacing;
use crate::ui::theme::Theme;
use eframe::egui;

pub(crate) struct AgentPaneContext<'a> {
    pub selected_agent: &'a SelectedAgent,
    pub selected_repo_id: Option<&'a String>,
    pub linked_repos: &'a [LinkedRepo],
    pub is_generating: bool,
    pub generation_error: Option<&'a String>,
    pub latest_plan: Option<&'a Plan>,
    pub run_enabled: bool,
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

            if let Some(err) = ctx.generation_error {
                ui.add_space(spacing::SPACING_XS);
                error_banner(ui, err);
            }

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

                ui.add_space(spacing::SPACING_SM);

                // 2. Buttons
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = spacing::SPACING_SM;

                    let reset_width = 80.0;
                    let run_width =
                        ui.available_width() - reset_width - ui.spacing().item_spacing.x;

                    let btn = cyber_button(
                        ui,
                        "RUN AGENT",
                        ctx.run_enabled,
                        ctx.is_generating,
                        None,
                        Some(run_width),
                    );

                    if btn.clicked() && ctx.run_enabled {
                        action_out = Some(GenerateAction::RunRequested);
                    }

                    let reset_btn = cyber_button(
                        ui,
                        "RESET",
                        true,
                        false,
                        Some(egui::Color32::from_rgb(200, 60, 60)),
                        Some(reset_width),
                    );

                    if reset_btn.clicked() {
                        action_out = Some(GenerateAction::Reset);
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
