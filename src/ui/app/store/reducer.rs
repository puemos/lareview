use super::action::{Action, AsyncAction, GenerateAction};
use super::command::Command;
use crate::ui::app::GenMsg;

use super::super::state::{AppState, AppView};

pub fn reduce(state: &mut AppState, action: Action) -> Vec<Command> {
    match action {
        Action::Generate(action) => reduce_generate(state, action),
        Action::Async(action) => reduce_async(state, action),
    }
}

fn reduce_generate(state: &mut AppState, action: GenerateAction) -> Vec<Command> {
    match action {
        GenerateAction::Reset => {
            state.diff_text.clear();
            state.generation_error = None;
            state.is_generating = false;
            state.reset_agent_timeline();
            state.current_view = AppView::Generate;
            Vec::new()
        }
        GenerateAction::RunRequested => {
            if state.diff_text.trim().is_empty() {
                state.generation_error = Some("Please paste a git diff first".into());
                return Vec::new();
            }

            state.generation_error = None;
            state.is_generating = true;
            state.reset_agent_timeline();
            vec![Command::StartGeneration {
                pull_request: Box::new(pull_request_from_state(state)),
                diff_text: state.diff_text.clone(),
                selected_agent_id: state.selected_agent.id.clone(),
            }]
        }
    }
}

fn reduce_async(state: &mut AppState, action: AsyncAction) -> Vec<Command> {
    match action {
        AsyncAction::GenerationMessage(msg) => reduce_generation_msg(state, msg),
    }
}

fn reduce_generation_msg(state: &mut AppState, msg: GenMsg) -> Vec<Command> {
    match msg {
        GenMsg::Progress(evt) => {
            state.ingest_progress(*evt);
            Vec::new()
        }
        GenMsg::Done(result) => {
            state.is_generating = false;

            match result {
                Ok(_payload) => vec![Command::SyncReviewAfterGeneration],
                Err(err) => {
                    state.generation_error = Some(err);
                    Vec::new()
                }
            }
        }
    }
}

fn pull_request_from_state(state: &AppState) -> crate::domain::PullRequest {
    if let Some(selected_pr_id) = &state.selected_pr_id
        && let Some(pr) = state.prs.iter().find(|p| &p.id == selected_pr_id)
    {
        return pr.clone();
    }
    crate::domain::PullRequest {
        id: state.pr_id.clone(),
        title: state.pr_title.clone(),
        repo: state.pr_repo.clone(),
        author: state.pr_author.clone(),
        branch: state.pr_branch.clone(),
        description: None,
        created_at: String::new(),
    }
}
