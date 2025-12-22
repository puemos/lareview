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
    }
}
