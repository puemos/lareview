use super::super::super::LaReviewApp;
use super::super::action::{Action, AsyncAction};
use super::super::command::D2Command;
use crate::ui::app::{GhMsg, GhStatusPayload};

pub fn check_github_status(app: &mut LaReviewApp) {
    let gh_tx = app.gh_tx.clone();

    tokio::spawn(async move {
        let result: Result<GhStatusPayload, String> = async {
            let gh_path = crate::infra::shell::find_bin("gh")
                .ok_or_else(|| "gh is not installed".to_string())?;

            let auth = tokio::process::Command::new(&gh_path)
                .args(["auth", "status", "--hostname", "github.com"])
                .output()
                .await
                .map_err(|e| format!("Failed to run `gh auth status`: {e}"))?;

            if !auth.status.success() {
                let stderr = String::from_utf8_lossy(&auth.stderr).trim().to_string();
                return Err(if stderr.is_empty() {
                    "Not authenticated. Run: gh auth login".to_string()
                } else {
                    format!("Not authenticated. gh: {stderr}")
                });
            }

            let whoami = tokio::process::Command::new(&gh_path)
                .args(["api", "user", "-q", ".login"])
                .output()
                .await
                .map_err(|e| format!("Failed to run `gh api user`: {e}"))?;

            let login = if whoami.status.success() {
                String::from_utf8(whoami.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            } else {
                None
            };

            Ok(GhStatusPayload {
                gh_path: gh_path.display().to_string(),
                login,
            })
        }
        .await;

        let _ = gh_tx.send(GhMsg::Done(result)).await;
    });
}

pub fn run_d2_command(app: &mut LaReviewApp, command: D2Command) {
    let command_str = match command {
        D2Command::Install => "curl -fsSL https://d2lang.com/install.sh | sh -s --",
        D2Command::Uninstall => "curl -fsSL https://d2lang.com/install.sh | sh -s -- --uninstall",
    }
    .to_string();

    let d2_install_tx = app.d2_install_tx.clone();

    crate::spawn(async move {
        let mut child = match tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command_str)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                let _ = d2_install_tx
                    .send(format!("Failed to spawn D2 process: {e}"))
                    .await;
                return;
            }
        };

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        use tokio::io::AsyncBufReadExt;
        let mut stdout_reader = tokio::io::BufReader::new(stdout).lines();
        let mut stderr_reader = tokio::io::BufReader::new(stderr).lines();

        loop {
            tokio::select! {
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => { let _ = d2_install_tx.send(line).await; }
                        _ => break,
                    }
                }
                line = stderr_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => { let _ = d2_install_tx.send(line).await; }
                        _ => break,
                    }
                }
            }
        }

        let _ = d2_install_tx
            .send("___INSTALL_COMPLETE___".to_string())
            .await;
    });
}

pub fn save_repo(app: &mut LaReviewApp, repo: crate::domain::LinkedRepo) {
    let repo_repo = app.repo_repo.clone();
    let action_tx = app.action_tx.clone();

    tokio::spawn(async move {
        let result = repo_repo
            .save(&repo)
            .map(|_| repo.clone())
            .map_err(|e| e.to_string());
        let _ = action_tx
            .send(Action::Async(AsyncAction::RepoSaved(result)))
            .await;
    });
}

pub fn delete_repo(app: &mut LaReviewApp, repo_id: String) {
    let repo_repo = app.repo_repo.clone();
    let action_tx = app.action_tx.clone();

    tokio::spawn(async move {
        let result = repo_repo
            .delete(&repo_id)
            .map(|_| repo_id.clone())
            .map_err(|e| e.to_string());
        let _ = action_tx
            .send(Action::Async(AsyncAction::RepoDeleted(result)))
            .await;
    });
}

pub fn pick_folder_for_link(app: &mut LaReviewApp) {
    let action_tx = app.action_tx.clone();

    tokio::spawn(async move {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            let remotes = crate::infra::vcs::git::extract_git_remotes(&path);

            let repo = crate::domain::LinkedRepo {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                path,
                remotes,
                created_at: chrono::Utc::now().to_rfc3339(),
            };

            let _ = action_tx
                .send(Action::Async(AsyncAction::NewRepoPicked(repo)))
                .await;
        }
    });
}

pub fn save_app_config_full(
    has_seen_requirements: bool,
    custom_agents: Vec<crate::infra::app_config::CustomAgentConfig>,
    agent_path_overrides: std::collections::HashMap<String, String>,
    agent_envs: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    preferred_editor_id: Option<String>,
) {
    let config = crate::infra::app_config::AppConfig {
        has_seen_requirements,
        custom_agents,
        agent_path_overrides,
        agent_envs,
        preferred_editor_id,
    };

    if let Err(err) = crate::infra::app_config::save_config(&config) {
        eprintln!("[config] Failed to save config: {err}");
    }
}
