use super::super::super::state::{SessionState, UiState};
use super::super::action::SettingsAction;
use super::super::command::{Command, D2Command};

pub fn reduce(
    ui: &mut UiState,
    session: &mut SessionState,
    action: SettingsAction,
) -> Vec<Command> {
    match action {
        SettingsAction::SetAllowD2Install(allow) => {
            ui.allow_d2_install = allow;
            Vec::new()
        }
        SettingsAction::CheckGitHubStatus => {
            if session.is_gh_status_checking {
                return Vec::new();
            }
            session.is_gh_status_checking = true;
            session.gh_status_error = None;
            vec![Command::CheckGitHubStatus]
        }
        SettingsAction::RequestD2Install => {
            if !ui.allow_d2_install || ui.is_d2_installing {
                return Vec::new();
            }
            ui.is_d2_installing = true;
            ui.d2_install_output.clear();
            vec![Command::RunD2 {
                command: D2Command::Install,
            }]
        }
        SettingsAction::RequestD2Uninstall => {
            if !ui.allow_d2_install || ui.is_d2_installing {
                return Vec::new();
            }
            ui.is_d2_installing = true;
            ui.d2_install_output.clear();
            vec![Command::RunD2 {
                command: D2Command::Uninstall,
            }]
        }
        SettingsAction::LinkRepository => {
            vec![Command::PickFolderForLink]
        }
        SettingsAction::UnlinkRepository(repo_id) => {
            vec![Command::DeleteRepo { repo_id }]
        }
        SettingsAction::DismissRequirements => {
            if matches!(
                ui.active_overlay,
                Some(crate::ui::app::OverlayState::Requirements)
            ) {
                ui.active_overlay = None;
            }
            ui.has_seen_requirements = true;
            vec![Command::SaveAppConfigFull {
                has_seen_requirements: ui.has_seen_requirements,
                custom_agents: ui.custom_agents.clone(),
                agent_path_overrides: ui.agent_path_overrides.clone(),
                agent_envs: ui.agent_envs.clone(),
                preferred_editor_id: ui.preferred_editor_id.clone(),
            }]
        }
        SettingsAction::SetPreferredEditor(editor_id) => {
            ui.preferred_editor_id = Some(editor_id.clone());
            if matches!(
                ui.active_overlay,
                Some(crate::ui::app::OverlayState::EditorPicker)
            ) {
                ui.active_overlay = None;
            }
            ui.editor_picker_error = None;

            let mut commands = vec![Command::SaveAppConfigFull {
                has_seen_requirements: ui.has_seen_requirements,
                custom_agents: ui.custom_agents.clone(),
                agent_path_overrides: ui.agent_path_overrides.clone(),
                agent_envs: ui.agent_envs.clone(),
                preferred_editor_id: ui.preferred_editor_id.clone(),
            }];

            if let Some(request) = ui.pending_editor_open.take() {
                commands.push(Command::OpenInEditor {
                    editor_id,
                    file_path: request.file_path,
                    line_number: request.line_number,
                });
            }

            commands
        }

        SettingsAction::ClearPreferredEditor => {
            ui.preferred_editor_id = None;
            if matches!(
                ui.active_overlay,
                Some(crate::ui::app::OverlayState::EditorPicker)
            ) {
                ui.active_overlay = None;
            }
            ui.editor_picker_error = None;
            vec![Command::SaveAppConfigFull {
                has_seen_requirements: ui.has_seen_requirements,
                custom_agents: ui.custom_agents.clone(),
                agent_path_overrides: ui.agent_path_overrides.clone(),
                agent_envs: ui.agent_envs.clone(),
                preferred_editor_id: ui.preferred_editor_id.clone(),
            }]
        }

        SettingsAction::UpdateAgentPath(agent_id, path) => {
            if path.trim().is_empty() {
                ui.agent_path_overrides.remove(&agent_id);
            } else {
                ui.agent_path_overrides.insert(agent_id, path);
            }
            Vec::new()
        }
        SettingsAction::AddCustomAgent(agent) => {
            ui.custom_agents.push(agent);
            vec![Command::SaveAppConfigFull {
                has_seen_requirements: ui.has_seen_requirements,
                custom_agents: ui.custom_agents.clone(),
                agent_path_overrides: ui.agent_path_overrides.clone(),
                agent_envs: ui.agent_envs.clone(),
                preferred_editor_id: ui.preferred_editor_id.clone(),
            }]
        }
        SettingsAction::DeleteCustomAgent(agent_id) => {
            ui.custom_agents.retain(|a| a.id != agent_id);
            vec![Command::SaveAppConfigFull {
                has_seen_requirements: ui.has_seen_requirements,
                custom_agents: ui.custom_agents.clone(),
                agent_path_overrides: ui.agent_path_overrides.clone(),
                agent_envs: ui.agent_envs.clone(),
                preferred_editor_id: ui.preferred_editor_id.clone(),
            }]
        }
        SettingsAction::UpdateAgentEnv(agent_id, key, value) => {
            ui.agent_envs
                .entry(agent_id)
                .or_default()
                .insert(key, value);
            Vec::new()
        }
        SettingsAction::RemoveAgentEnv(agent_id, key) => {
            if let Some(envs) = ui.agent_envs.get_mut(&agent_id) {
                envs.remove(&key);
                if envs.is_empty() {
                    ui.agent_envs.remove(&agent_id);
                }
            }
            Vec::new()
        }
        SettingsAction::SaveAgentSettings => {
            vec![Command::SaveAppConfigFull {
                has_seen_requirements: ui.has_seen_requirements,
                custom_agents: ui.custom_agents.clone(),
                agent_path_overrides: ui.agent_path_overrides.clone(),
                agent_envs: ui.agent_envs.clone(),
                preferred_editor_id: ui.preferred_editor_id.clone(),
            }]
        }
        SettingsAction::LoadAgentSettings => {
            let config = crate::infra::app_config::load_config();
            ui.agent_path_overrides = config.agent_path_overrides;
            ui.custom_agents = config.custom_agents;
            ui.agent_envs = config.agent_envs;
            ui.preferred_editor_id = config.preferred_editor_id;
            Vec::new()
        }
    }
}
