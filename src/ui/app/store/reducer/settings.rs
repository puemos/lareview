use super::super::super::state::AppState;
use super::super::action::SettingsAction;
use super::super::command::{Command, D2Command};

pub fn reduce(state: &mut AppState, action: SettingsAction) -> Vec<Command> {
    match action {
        SettingsAction::SetAllowD2Install(allow) => {
            state.allow_d2_install = allow;
            Vec::new()
        }
        SettingsAction::CheckGitHubStatus => {
            if state.is_gh_status_checking {
                return Vec::new();
            }
            state.is_gh_status_checking = true;
            state.gh_status_error = None;
            vec![Command::CheckGitHubStatus]
        }
        SettingsAction::RequestD2Install => {
            if !state.allow_d2_install || state.is_d2_installing {
                return Vec::new();
            }
            state.is_d2_installing = true;
            state.d2_install_output.clear();
            vec![Command::RunD2 {
                command: D2Command::Install,
            }]
        }
        SettingsAction::RequestD2Uninstall => {
            if !state.allow_d2_install || state.is_d2_installing {
                return Vec::new();
            }
            state.is_d2_installing = true;
            state.d2_install_output.clear();
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
            state.extra_path = extra_path;
            Vec::new()
        }
        SettingsAction::SaveExtraPath => vec![Command::SaveAppConfig {
            extra_path: state.extra_path.clone(),
            has_seen_requirements: state.has_seen_requirements,
        }],
        SettingsAction::DismissRequirements => {
            state.show_requirements_modal = false;
            state.has_seen_requirements = true;
            vec![Command::SaveAppConfig {
                extra_path: state.extra_path.clone(),
                has_seen_requirements: state.has_seen_requirements,
            }]
        }
    }
}
