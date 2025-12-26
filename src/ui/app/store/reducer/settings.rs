use super::super::super::state::{AgentSettingsSnapshot, SessionState, UiState};
use super::super::action::SettingsAction;
use super::super::command::{Command, D2Command};

fn snapshot_for_agent(ui: &UiState, agent_id: &str) -> AgentSettingsSnapshot {
    let path_override = ui.agent_path_overrides.get(agent_id).and_then(|path| {
        if path.is_empty() {
            None
        } else {
            Some(path.clone())
        }
    });
    AgentSettingsSnapshot {
        agent_id: agent_id.to_string(),
        path_override,
        envs: ui.agent_envs.get(agent_id).and_then(|envs| {
            if envs.is_empty() {
                None
            } else {
                Some(envs.clone())
            }
        }),
    }
}

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
        SettingsAction::UpdateExtraPath(extra_path) => {
            ui.extra_path = extra_path;
            Vec::new()
        }
        SettingsAction::SaveExtraPath => vec![Command::SaveAppConfig {
            extra_path: ui.extra_path.clone(),
            has_seen_requirements: ui.has_seen_requirements,
        }],
        SettingsAction::DismissRequirements => {
            ui.show_requirements_modal = false;
            ui.has_seen_requirements = true;
            vec![Command::SaveAppConfig {
                extra_path: ui.extra_path.clone(),
                has_seen_requirements: ui.has_seen_requirements,
            }]
        }
        SettingsAction::UpdateAgentPath(agent_id, path) => {
            if path.trim().is_empty() {
                ui.agent_path_overrides.remove(&agent_id);
            } else {
                ui.agent_path_overrides.insert(agent_id, path);
            }
            ui.is_agent_settings_modified = true;
            Vec::new()
        }
        SettingsAction::AddCustomAgent(agent) => {
            ui.custom_agents.push(agent);
            ui.is_agent_settings_modified = true;
            Vec::new()
        }
        SettingsAction::RemoveCustomAgent(agent_id) => {
            ui.custom_agents.retain(|a| a.id != agent_id);
            ui.is_agent_settings_modified = true;
            Vec::new()
        }
        SettingsAction::UpdateAgentEnv(agent_id, key, value) => {
            ui.agent_envs
                .entry(agent_id)
                .or_default()
                .insert(key, value);
            ui.is_agent_settings_modified = true;
            Vec::new()
        }
        SettingsAction::RemoveAgentEnv(agent_id, key) => {
            if let Some(envs) = ui.agent_envs.get_mut(&agent_id) {
                envs.remove(&key);
                if envs.is_empty() {
                    ui.agent_envs.remove(&agent_id);
                }
            }
            ui.is_agent_settings_modified = true;
            Vec::new()
        }
        SettingsAction::SaveAgentSettings => {
            ui.is_agent_settings_modified = false;
            if let Some(agent_id) = ui.editing_agent_id.clone() {
                ui.agent_settings_snapshot = Some(snapshot_for_agent(ui, &agent_id));
            }
            vec![Command::SaveAppConfigFull {
                extra_path: ui.extra_path.clone(),
                has_seen_requirements: ui.has_seen_requirements,
                custom_agents: ui.custom_agents.clone(),
                agent_path_overrides: ui.agent_path_overrides.clone(),
                agent_envs: ui.agent_envs.clone(),
            }]
        }
        SettingsAction::LoadAgentSettings => {
            let config = crate::infra::app_config::load_config();
            ui.agent_path_overrides = config.agent_path_overrides;
            ui.custom_agents = config.custom_agents;
            ui.agent_envs = config.agent_envs;
            ui.is_agent_settings_modified = false;
            ui.agent_settings_snapshot = None;
            Vec::new()
        }
        SettingsAction::OpenAgentSettings(agent_id) => {
            ui.editing_agent_id = Some(agent_id.clone());
            ui.agent_settings_snapshot = Some(snapshot_for_agent(ui, &agent_id));
            Vec::new()
        }
        SettingsAction::CloseAgentSettings => {
            let editing_agent_id = ui.editing_agent_id.take();
            if let (Some(agent_id), Some(snapshot)) =
                (editing_agent_id, ui.agent_settings_snapshot.take())
                && snapshot.agent_id == agent_id
            {
                match snapshot.path_override {
                    Some(path) => {
                        ui.agent_path_overrides.insert(agent_id.clone(), path);
                    }
                    None => {
                        ui.agent_path_overrides.remove(&agent_id);
                    }
                }
                match snapshot.envs {
                    Some(envs) => {
                        ui.agent_envs.insert(agent_id, envs);
                    }
                    None => {
                        ui.agent_envs.remove(&agent_id);
                    }
                }
            }
            Vec::new()
        }
        SettingsAction::OpenAddCustomAgent => {
            ui.show_add_custom_agent_modal = true;
            ui.custom_agent_draft = Default::default();
            Vec::new()
        }
        SettingsAction::CloseAddCustomAgent => {
            ui.show_add_custom_agent_modal = false;
            Vec::new()
        }
    }
}
