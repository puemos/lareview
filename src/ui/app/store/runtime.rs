use super::command::Command;
use crate::infra::acp::{GenerateTasksInput, generate_tasks_with_acp, list_agent_candidates};
use crate::ui::app::{GenMsg, GenResultPayload};

use super::super::LaReviewApp;
use super::super::state::AppView;

pub fn run(app: &mut LaReviewApp, command: Command) {
    match command {
        Command::StartGeneration {
            pull_request,
            diff_text,
            selected_agent_id,
        } => start_generation(app, *pull_request, diff_text, selected_agent_id),
        Command::SyncReviewAfterGeneration => sync_review_after_generation(app),
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

fn sync_review_after_generation(app: &mut LaReviewApp) {
    app.sync_review_from_db();
    let has_tasks_for_selected_pr = !app.state.tasks().is_empty();
    if has_tasks_for_selected_pr {
        app.state.current_view = AppView::Review;
        app.state.generation_error = None;
    } else {
        app.state.current_view = AppView::Generate;
        app.state.generation_error = Some("No tasks generated".to_string());
    }
}
