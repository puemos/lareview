use super::super::super::LaReviewApp;
use crate::infra::acp::{GenerateTasksInput, generate_tasks_with_acp, list_agent_candidates};
use crate::ui::app::{GenMsg, GenResultPayload};

pub fn resolve_generate_input(
    app: &mut LaReviewApp,
    input_text: String,
    selected_agent_id: String,
    review_id: Option<String>,
) {
    let gen_tx = app.gen_tx.clone();

    tokio::spawn(async move {
        let result = crate::ui::app::generate_input::resolve_generate_input(
            input_text,
            selected_agent_id,
            review_id,
        )
        .await
        .map_err(|e| e.to_string());

        let _ = gen_tx
            .send(crate::ui::app::GenMsg::InputResolved(Box::new(result)))
            .await;
    });
}

pub fn fetch_pr_context_preview(app: &mut LaReviewApp, input_ref: String) {
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

pub fn refresh_github_review(app: &mut LaReviewApp, review_id: String, selected_agent_id: String) {
    let gen_tx = app.gen_tx.clone();
    let review = app
        .state
        .domain
        .reviews
        .iter()
        .find(|r| r.id == review_id)
        .cloned();

    tokio::spawn(async move {
        let Some(review) = review else {
            return;
        };
        let result =
            crate::ui::app::generate_input::resolve_github_refresh(&review, selected_agent_id)
                .await
                .map_err(|e| e.to_string());

        let _ = gen_tx
            .send(crate::ui::app::GenMsg::InputResolved(Box::new(result)))
            .await;
    });
}

pub fn abort_generation(app: &mut LaReviewApp) {
    if let Some(token) = app.agent_cancel_token.take() {
        token.cancel();
    }
    if let Some(task) = app.agent_task.take() {
        task.abort();
    }
}

pub fn start_generation(
    app: &mut LaReviewApp,
    run_context: crate::infra::acp::RunContext,
    selected_agent_id: String,
) {
    // Abort existing if any
    if let Some(token) = app.agent_cancel_token.take() {
        token.cancel();
    }
    if let Some(task) = app.agent_task.take() {
        task.abort();
    }

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
    let cancel_token = tokio_util::sync::CancellationToken::new();

    let start_log = format!(
        "agent: {} ({} {})",
        candidate.id,
        agent_command,
        candidate.args.join(" ")
    );

    let repo_root = app
        .state
        .ui
        .selected_repo_id
        .as_ref()
        .and_then(|id| app.state.domain.linked_repos.iter().find(|r| &r.id == id))
        .map(|r| r.path.clone());

    let input = GenerateTasksInput {
        run_context,
        repo_root,
        agent_command,
        agent_args: candidate.args,
        progress_tx: Some(progress_tx),
        mcp_server_binary: None,
        timeout_secs: Some(5000),
        cancel_token: Some(cancel_token.clone()),
        debug: std::env::var("ACP_DEBUG").is_ok(),
    };

    let gen_tx = app.gen_tx.clone();

    let task = tokio::spawn(async move {
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

    app.agent_task = Some(task);
    app.agent_cancel_token = Some(cancel_token);
}
