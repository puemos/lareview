use super::action::{Action, AsyncAction, ReviewDataPayload};
use super::command::{Command, D2Command};
use crate::infra::acp::{GenerateTasksInput, generate_tasks_with_acp, list_agent_candidates};
use crate::ui::app::{GenMsg, GenResultPayload};

use super::super::LaReviewApp;

pub fn run(app: &mut LaReviewApp, command: Command) {
    match command {
        Command::StartGeneration {
            pull_request,
            diff_text,
            selected_agent_id,
        } => start_generation(app, *pull_request, diff_text, selected_agent_id),
        Command::RefreshReviewData { reason } => refresh_review_data(app, reason),
        Command::LoadTaskNote { task_id } => load_task_note(app, task_id),
        Command::UpdateTaskStatus { task_id, status } => update_task_status(app, task_id, status),
        Command::CleanDoneTasks { pr_id } => clean_done_tasks(app, pr_id),
        Command::SaveNote {
            task_id,
            body,
            file_path,
            line_number,
        } => save_note(app, task_id, body, file_path, line_number),
        Command::RunD2 { command } => run_d2_command(app, command),
    }
}

fn start_generation(
    app: &mut LaReviewApp,
    pull_request: crate::domain::PullRequest,
    diff_text: String,
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
        pull_request,
        diff_text,
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
        let prs = app
            .pr_repo
            .list_all()
            .map_err(|e| format!("Failed to load pull requests: {e}"))?;
        let tasks = app
            .task_repo
            .find_all()
            .map_err(|e| format!("Failed to load tasks: {e}"))?;
        Ok(ReviewDataPayload { prs, tasks })
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

fn clean_done_tasks(app: &mut LaReviewApp, pr_id: Option<String>) {
    let Some(pr_id) = pr_id else {
        app.dispatch(Action::Async(AsyncAction::DoneTasksCleaned(Err(
            "No pull request selected".into(),
        ))));
        return;
    };

    let done_ids = match app.task_repo.find_done_ids_by_pr(&pr_id) {
        Ok(ids) => ids,
        Err(err) => {
            app.dispatch(Action::Async(AsyncAction::DoneTasksCleaned(Err(format!(
                "Failed to list done tasks: {err}"
            )))));
            return;
        }
    };

    if done_ids.is_empty() {
        return;
    }

    let result = (|| -> Result<(), String> {
        app.note_repo
            .delete_by_task_ids(&done_ids)
            .map_err(|e| format!("Failed to delete notes for done tasks: {e}"))?;

        app.task_repo
            .delete_by_ids(&done_ids)
            .map_err(|e| format!("Failed to delete done tasks: {e}"))?;

        Ok(())
    })();

    app.dispatch(Action::Async(AsyncAction::DoneTasksCleaned(result)));
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
