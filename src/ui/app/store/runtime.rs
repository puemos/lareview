use super::action::{Action, AsyncAction, ReviewDataPayload};
use super::command::{Command, D2Command};
use crate::domain::ReviewId;
use crate::infra::acp::{GenerateTasksInput, generate_tasks_with_acp, list_agent_candidates};
use crate::ui::app::{GenMsg, GenResultPayload};
use crate::ui::app::{GhMsg, GhStatusPayload};

use super::super::LaReviewApp;

pub fn run(app: &mut LaReviewApp, command: Command) {
    match command {
        Command::ResolveGenerateInput {
            input_text,
            selected_agent_id,
        } => resolve_generate_input(app, input_text, selected_agent_id),
        Command::FetchPrContextPreview { input_ref } => fetch_pr_context_preview(app, input_ref),
        Command::CheckGitHubStatus => check_github_status(app),
        Command::RefreshGitHubReview {
            review_id,
            selected_agent_id,
        } => refresh_github_review(app, review_id, selected_agent_id),
        Command::StartGeneration {
            run_context,
            selected_agent_id,
        } => start_generation(app, *run_context, selected_agent_id),
        Command::RefreshReviewData { reason } => refresh_review_data(app, reason),
        Command::LoadTaskNote { task_id } => load_task_note(app, task_id),
        Command::UpdateTaskStatus { task_id, status } => update_task_status(app, task_id, status),
        Command::DeleteReview { review_id } => delete_review(app, review_id),
        Command::SaveNote {
            task_id,
            body,
            file_path,
            line_number,
        } => save_note(app, task_id, body, file_path, line_number),
        Command::RunD2 { command } => run_d2_command(app, command),
    }
}

fn resolve_generate_input(app: &mut LaReviewApp, input_text: String, selected_agent_id: String) {
    let gen_tx = app.gen_tx.clone();

    tokio::spawn(async move {
        let result =
            crate::ui::app::generate_input::resolve_generate_input(input_text, selected_agent_id)
                .await
                .map_err(|e| e.to_string());

        let _ = gen_tx
            .send(crate::ui::app::GenMsg::InputResolved(Box::new(result)))
            .await;
    });
}

fn fetch_pr_context_preview(app: &mut LaReviewApp, input_ref: String) {
    let gen_tx = app.gen_tx.clone();

    tokio::spawn(async move {
        let result = crate::ui::app::generate_input::resolve_pr_preview(input_ref.clone())
            .await
            .map_err(|e| e.to_string());

        let _ = gen_tx
            .send(crate::ui::app::GenMsg::PreviewResolved { input_ref, result })
            .await;
    });
}

fn refresh_github_review(app: &mut LaReviewApp, review_id: String, selected_agent_id: String) {
    let Some(review) = app
        .state
        .reviews
        .iter()
        .find(|r| r.id == review_id)
        .cloned()
    else {
        let _ = app
            .gen_tx
            .try_send(crate::ui::app::GenMsg::InputResolved(Box::new(Err(
                "Selected review not found".into(),
            ))));
        return;
    };

    let gen_tx = app.gen_tx.clone();

    tokio::spawn(async move {
        let result =
            crate::ui::app::generate_input::resolve_github_refresh(&review, selected_agent_id)
                .await
                .map_err(|e| e.to_string());

        let _ = gen_tx
            .send(crate::ui::app::GenMsg::InputResolved(Box::new(result)))
            .await;
    });
}

fn check_github_status(app: &mut LaReviewApp) {
    let gh_tx = app.gh_tx.clone();

    tokio::spawn(async move {
        let result: Result<GhStatusPayload, String> = async {
            let gh_path = which::which("gh").map_err(|_| "gh is not installed".to_string())?;

            let auth = tokio::process::Command::new("gh")
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

            let whoami = tokio::process::Command::new("gh")
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

fn start_generation(
    app: &mut LaReviewApp,
    run_context: crate::infra::acp::RunContext,
    selected_agent_id: String,
) {
    let candidates: Vec<_> = list_agent_candidates()
        .into_iter()
        .filter(|c| c.available)
        .collect();

    let Some(candidate) = candidates
        .iter()
        .find(|c| c.id == selected_agent_id)
        .cloned()
        .or_else(|| candidates.first().cloned())
    else {
        let _ = app.gen_tx.try_send(GenMsg::Done(Err(
            "No ACP agents available on this system".into()
        )));
        return;
    };

    let Some(agent_command) = candidate.command.clone() else {
        let _ = app.gen_tx.try_send(GenMsg::Done(Err(format!(
            "Selected agent '{}' is not executable on this system",
            candidate.id
        ))));
        return;
    };

    let _ = app.gen_tx.try_send(GenMsg::Progress(Box::new(
        crate::infra::acp::ProgressEvent::LocalLog(format!("starting agent: {}", candidate.label)),
    )));

    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();

    let start_log = format!(
        "agent: {} ({} {})",
        candidate.id,
        agent_command,
        candidate.args.join(" ")
    );

    let input = GenerateTasksInput {
        run_context,
        repo_root: None,
        agent_command,
        agent_args: candidate.args,
        progress_tx: Some(progress_tx),
        mcp_server_binary: None,
        timeout_secs: Some(500),
        debug: false,
    };

    let gen_tx = app.gen_tx.clone();

    tokio::spawn(async move {
        let mut result_fut = std::pin::pin!(generate_tasks_with_acp(input));

        loop {
            tokio::select! {
                evt = progress_rx.recv() => {
                    if let Some(evt) = evt {
                        let _ = gen_tx.send(GenMsg::Progress(Box::new(evt))).await;
                    }
                }
                res = &mut result_fut => {
                    let msg = match res {
                        Ok(res) => {
                            let mut logs = res.logs;
                            logs.insert(0, start_log.clone());
                            GenMsg::Done(Ok(GenResultPayload {
                                messages: res.messages,
                                thoughts: res.thoughts,
                                logs,
                            }))
                        }
                        Err(e) => GenMsg::Done(Err(format!("Generation failed: {e}"))),
                    };

                    let _ = gen_tx.send(msg).await;
                    break;
                }
            }
        }
    });
}

fn refresh_review_data(app: &mut LaReviewApp, reason: super::command::ReviewDataRefreshReason) {
    let result = (|| -> Result<ReviewDataPayload, String> {
        let reviews = app
            .review_repo
            .list_all()
            .map_err(|e| format!("Failed to load reviews: {e}"))?;
        let runs = app
            .run_repo
            .list_all()
            .map_err(|e| format!("Failed to load review runs: {e}"))?;
        let tasks = app
            .task_repo
            .find_all()
            .map_err(|e| format!("Failed to load tasks: {e}"))?;
        Ok(ReviewDataPayload {
            reviews,
            runs,
            tasks,
        })
    })();

    app.dispatch(Action::Async(AsyncAction::ReviewDataLoaded {
        reason,
        result,
    }));
}

fn load_task_note(app: &mut LaReviewApp, task_id: crate::domain::TaskId) {
    let result = app
        .note_repo
        .find_by_task(&task_id)
        .map(|opt| opt.map(|n| n.body))
        .map_err(|e| format!("Failed to load note: {e}"));

    match result {
        Ok(note) => {
            app.dispatch(Action::Async(AsyncAction::TaskNoteLoaded { task_id, note }));
        }
        Err(err) => {
            app.dispatch(Action::Async(AsyncAction::TaskNoteLoaded {
                task_id: task_id.clone(),
                note: None,
            }));
            app.dispatch(Action::Async(AsyncAction::NoteSaved(Err(err))));
        }
    }
}

fn update_task_status(
    app: &mut LaReviewApp,
    task_id: crate::domain::TaskId,
    status: crate::domain::TaskStatus,
) {
    let result = app
        .task_repo
        .update_status(&task_id, status)
        .map_err(|e| format!("Failed to update task status: {e}"));

    app.dispatch(Action::Async(AsyncAction::TaskStatusSaved(result)));
}

fn delete_review(app: &mut LaReviewApp, review_id: ReviewId) {
    let result = (|| -> Result<(), String> {
        let runs = app
            .run_repo
            .find_by_review_id(&review_id)
            .map_err(|e| format!("Failed to fetch runs for review: {e}"))?;

        if !runs.is_empty() {
            let run_ids: Vec<_> = runs.iter().map(|r| r.id.clone()).collect();
            let tasks = app
                .task_repo
                .find_by_run_ids(&run_ids)
                .map_err(|e| format!("Failed to fetch tasks for runs: {e}"))?;

            if !tasks.is_empty() {
                let task_ids: Vec<_> = tasks.iter().map(|t| t.id.clone()).collect();

                app.note_repo
                    .delete_by_task_ids(&task_ids)
                    .map_err(|e| format!("Failed to delete notes: {e}"))?;

                app.task_repo
                    .delete_by_ids(&task_ids)
                    .map_err(|e| format!("Failed to delete tasks: {e}"))?;
            }

            app.run_repo
                .delete_by_review_id(&review_id)
                .map_err(|e| format!("Failed to delete runs: {e}"))?;
        }

        app.review_repo
            .delete(&review_id)
            .map_err(|e| format!("Failed to delete review: {e}"))?;

        Ok(())
    })();
    app.dispatch(Action::Async(AsyncAction::ReviewDeleted(result)));
}

fn save_note(
    app: &mut LaReviewApp,
    task_id: crate::domain::TaskId,
    body: String,
    file_path: Option<String>,
    line_number: Option<u32>,
) {
    let note = crate::domain::Note {
        task_id,
        body,
        updated_at: chrono::Utc::now().to_rfc3339(),
        file_path,
        line_number,
    };

    let result = app
        .note_repo
        .save(&note)
        .map_err(|e| format!("Failed to save note: {e}"));

    app.dispatch(Action::Async(AsyncAction::NoteSaved(result)));
}

fn run_d2_command(app: &mut LaReviewApp, command: D2Command) {
    let command_str = match command {
        D2Command::Install => "curl -fsSL https://d2lang.com/install.sh | sh -s --",
        D2Command::Uninstall => "curl -fsSL https://d2lang.com/install.sh | sh -s -- --uninstall",
    }
    .to_string();

    let d2_install_tx = app.d2_install_tx.clone();

    crate::RUNTIME.get().unwrap().spawn(async move {
        let mut child = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command_str)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn D2 process");

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
