use super::super::super::state::{AppState, AppView};
use super::super::action::GenerateAction;
use super::super::command::{Command, ReviewDataRefreshReason};
use crate::ui::app::GenMsg;

pub fn reduce(state: &mut AppState, action: GenerateAction) -> Vec<Command> {
    match action {
        GenerateAction::Reset => {
            state.diff_text.clear();
            state.generate_preview = None;
            state.is_preview_fetching = false;
            state.last_preview_input_ref = None;
            state.generation_error = None;
            state.is_generating = false;
            state.reset_agent_timeline();
            state.current_view = AppView::Generate;
            Vec::new()
        }
        GenerateAction::RunRequested => {
            if state.diff_text.trim().is_empty() {
                state.generation_error =
                    Some("Please paste a diff or GitHub PR reference first".into());
                return Vec::new();
            }

            state.generation_error = None;
            state.is_generating = true;
            state.reset_agent_timeline();
            state.generate_preview = None;
            vec![Command::ResolveGenerateInput {
                input_text: state.diff_text.clone(),
                selected_agent_id: state.selected_agent.id.clone(),
                review_id: None,
            }]
        }

        GenerateAction::FetchPrContext(input_ref) => {
            let input_ref = input_ref.trim().to_string();
            if input_ref.is_empty() {
                return Vec::new();
            }
            if state.is_preview_fetching {
                return Vec::new();
            }
            if state.last_preview_input_ref.as_deref() == Some(input_ref.as_str())
                && state.generate_preview.is_some()
            {
                return Vec::new();
            }

            state.is_preview_fetching = true;
            state.last_preview_input_ref = Some(input_ref.clone());
            state.generation_error = None;
            state.generate_preview = None;

            vec![Command::FetchPrContextPreview { input_ref }]
        }
        GenerateAction::SelectAgent(agent) => {
            state.selected_agent = agent;
            Vec::new()
        }
        GenerateAction::ClearTimeline => {
            state.reset_agent_timeline();
            Vec::new()
        }
        GenerateAction::SelectRepo(repo_id) => {
            state.selected_repo_id = repo_id;
            Vec::new()
        }
        GenerateAction::ToggleAgentPanel => {
            state.agent_panel_collapsed = !state.agent_panel_collapsed;
            Vec::new()
        }
        GenerateAction::TogglePlanPanel => {
            state.plan_panel_collapsed = !state.plan_panel_collapsed;
            Vec::new()
        }
    }
}

pub fn reduce_msg(state: &mut AppState, msg: GenMsg) -> Vec<Command> {
    match msg {
        GenMsg::PreviewResolved { input_ref, result } => {
            state.is_preview_fetching = false;
            if state.diff_text.trim() != input_ref.trim() {
                return Vec::new();
            }
            match result {
                Ok(preview) => {
                    state.generate_preview = Some(preview.clone());
                    state.generation_error = None;

                    if let Some(github) = &preview.github {
                        // Try to auto-select repo by matching "owner/repo" in remotes
                        let search_pattern = format!("{}/{}", github.pr.owner, github.pr.repo);
                        if let Some(matched_repo) = state.linked_repos.iter().find(|r| {
                            r.remotes
                                .iter()
                                .any(|remote: &String| remote.contains(&search_pattern))
                        }) {
                            state.selected_repo_id = Some(matched_repo.id.clone());
                        }
                    }
                }
                Err(err) => {
                    state.generate_preview = None;
                    state.generation_error = Some(err);
                }
            }
            Vec::new()
        }
        GenMsg::InputResolved(result) => match *result {
            Ok(payload) => {
                state.generate_preview = Some(payload.preview);
                state.selected_review_id = Some(payload.run_context.review_id.clone());
                state.selected_run_id = Some(payload.run_context.run_id.clone());
                vec![Command::StartGeneration {
                    run_context: Box::new(payload.run_context),
                    selected_agent_id: state.selected_agent.id.clone(),
                }]
            }
            Err(err) => {
                state.is_generating = false;
                state.generation_error = Some(err);
                Vec::new()
            }
        },
        GenMsg::Progress(evt) => {
            state.ingest_progress(*evt);
            Vec::new()
        }
        GenMsg::Done(result) => {
            state.is_generating = false;

            match result {
                Ok(_payload) => vec![Command::RefreshReviewData {
                    reason: ReviewDataRefreshReason::AfterGeneration,
                }],
                Err(err) => {
                    state.generation_error = Some(err);
                    Vec::new()
                }
            }
        }
    }
}
