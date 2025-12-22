use super::super::super::state::{AppState, AppView};
use super::super::action::GenerateAction;
use super::super::command::{Command, ReviewDataRefreshReason};
use crate::ui::app::GenMsg;

pub fn reduce(state: &mut AppState, action: GenerateAction) -> Vec<Command> {
    match action {
        GenerateAction::Reset => {
            state.session.diff_text.clear();
            state.session.generate_preview = None;
            state.session.is_preview_fetching = false;
            state.session.last_preview_input_ref = None;
            state.session.generation_error = None;
            state.session.is_generating = false;
            state.reset_agent_timeline();
            state.ui.current_view = AppView::Generate;
            Vec::new()
        }
        GenerateAction::RunRequested => {
            if state.session.diff_text.trim().is_empty() {
                state.session.generation_error =
                    Some("Please paste a diff or GitHub PR reference first".into());
                return Vec::new();
            }

            state.session.generation_error = None;
            state.session.is_generating = true;
            state.reset_agent_timeline();
            state.session.generate_preview = None;
            vec![Command::ResolveGenerateInput {
                input_text: state.session.diff_text.clone(),
                selected_agent_id: state.session.selected_agent.id.clone(),
                review_id: None,
            }]
        }

        GenerateAction::FetchPrContext(input_ref) => {
            let input_ref = input_ref.trim().to_string();
            if input_ref.is_empty() {
                return Vec::new();
            }
            if state.session.is_preview_fetching {
                return Vec::new();
            }
            if state.session.last_preview_input_ref.as_deref() == Some(input_ref.as_str())
                && state.session.generate_preview.is_some()
            {
                return Vec::new();
            }

            state.session.is_preview_fetching = true;
            state.session.last_preview_input_ref = Some(input_ref.clone());
            state.session.generation_error = None;
            state.session.generate_preview = None;

            vec![Command::FetchPrContextPreview { input_ref }]
        }
        GenerateAction::SelectAgent(agent) => {
            state.session.selected_agent = agent;
            Vec::new()
        }
        GenerateAction::ClearTimeline => {
            state.reset_agent_timeline();
            Vec::new()
        }
        GenerateAction::SelectRepo(repo_id) => {
            state.ui.selected_repo_id = repo_id;
            Vec::new()
        }
        GenerateAction::ToggleAgentPanel => {
            state.ui.agent_panel_collapsed = !state.ui.agent_panel_collapsed;
            Vec::new()
        }
        GenerateAction::TogglePlanPanel => {
            state.ui.plan_panel_collapsed = !state.ui.plan_panel_collapsed;
            Vec::new()
        }
    }
}

pub fn reduce_msg(state: &mut AppState, msg: GenMsg) -> Vec<Command> {
    match msg {
        GenMsg::PreviewResolved { input_ref, result } => {
            state.session.is_preview_fetching = false;
            if state.session.diff_text.trim() != input_ref.trim() {
                return Vec::new();
            }
            match result {
                Ok(preview) => {
                    state.session.generate_preview = Some(preview.clone());
                    state.session.generation_error = None;

                    if let Some(github) = &preview.github {
                        // Try to auto-select repo by matching "owner/repo" in remotes
                        let search_pattern = format!("{}/{}", github.pr.owner, github.pr.repo);
                        if let Some(matched_repo) = state.domain.linked_repos.iter().find(|r| {
                            r.remotes
                                .iter()
                                .any(|remote: &String| remote.contains(&search_pattern))
                        }) {
                            state.ui.selected_repo_id = Some(matched_repo.id.clone());
                        }
                    }
                }
                Err(err) => {
                    state.session.generate_preview = None;
                    state.session.generation_error = Some(err);
                }
            }
            Vec::new()
        }
        GenMsg::InputResolved(result) => match *result {
            Ok(payload) => {
                state.session.generate_preview = Some(payload.preview);
                state.ui.selected_review_id = Some(payload.run_context.review_id.clone());
                state.ui.selected_run_id = Some(payload.run_context.run_id.clone());
                vec![Command::StartGeneration {
                    run_context: Box::new(payload.run_context),
                    selected_agent_id: state.session.selected_agent.id.clone(),
                }]
            }
            Err(err) => {
                state.session.is_generating = false;
                state.session.generation_error = Some(err);
                Vec::new()
            }
        },
        GenMsg::Progress(evt) => {
            state.ingest_progress(*evt);
            Vec::new()
        }
        GenMsg::Done(result) => {
            state.session.is_generating = false;

            match result {
                Ok(_payload) => vec![Command::RefreshReviewData {
                    reason: ReviewDataRefreshReason::AfterGeneration,
                }],
                Err(err) => {
                    state.session.generation_error = Some(err);
                    Vec::new()
                }
            }
        }
    }
}
