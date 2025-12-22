use super::super::super::state::AppState;
use super::super::action::SettingsAction;
use super::super::command::{Command, D2Command};

pub fn reduce(state: &mut AppState, action: SettingsAction) -> Vec<Command> {
    match action {
        SettingsAction::SetAllowD2Install(allow) => {
            state.ui.allow_d2_install = allow;
            Vec::new()
        }
        SettingsAction::CheckGitHubStatus => {
            if state.session.is_gh_status_checking {
                return Vec::new();
            }
            state.session.is_gh_status_checking = true;
            state.session.gh_status_error = None;
            vec![Command::CheckGitHubStatus]
        }
        SettingsAction::RequestD2Install => {
            if !state.ui.allow_d2_install || state.ui.is_d2_installing {
                return Vec::new();
            }
            state.ui.is_d2_installing = true;
            state.ui.d2_install_output.clear();
            vec![Command::RunD2 {
                command: D2Command::Install,
            }]
        }
        SettingsAction::RequestD2Uninstall => {
            if !state.ui.allow_d2_install || state.ui.is_d2_installing {
                return Vec::new();
            }
            state.ui.is_d2_installing = true;
            state.ui.d2_install_output.clear();
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
            state.ui.extra_path = extra_path;
            Vec::new()
        }
        SettingsAction::SaveExtraPath => vec![Command::SaveAppConfig {
            extra_path: state.ui.extra_path.clone(),
            has_seen_requirements: state.ui.has_seen_requirements,
        }],
        SettingsAction::DismissRequirements => {
            state.ui.show_requirements_modal = false;
            state.ui.has_seen_requirements = true;
            vec![Command::SaveAppConfig {
                extra_path: state.ui.extra_path.clone(),
                has_seen_requirements: state.ui.has_seen_requirements,
            }]
        }
    }
}
