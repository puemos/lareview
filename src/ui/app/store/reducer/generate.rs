use super::super::action::GenerateAction;
use super::super::command::{Command, ReviewDataRefreshReason};
use crate::ui::app::GenMsg;
use crate::ui::app::state::{AppView, DomainState, SessionState, UiState};

pub fn reduce(
    ui: &mut UiState,
    session: &mut SessionState,
    action: GenerateAction,
) -> Vec<Command> {
    match action {
        GenerateAction::Reset => {
            session.diff_text.clear();
            session.generate_preview = None;
            session.is_preview_fetching = false;
            session.last_preview_input_ref = None;
            session.generation_error = None;
            session.is_generating = false;
            session.reset_agent_timeline();
            ui.current_view = AppView::Generate;
            Vec::new()
        }
        GenerateAction::RunRequested => {
            if session.diff_text.trim().is_empty() {
                session.generation_error =
                    Some("Please paste a diff or GitHub PR reference first".into());
                return Vec::new();
            }

            session.generation_error = None;
            session.is_generating = true;
            session.reset_agent_timeline();
            session.generate_preview = None;
            vec![Command::ResolveGenerateInput {
                input_text: session.diff_text.clone(),
                selected_agent_id: session.selected_agent.id.clone(),
                review_id: None,
            }]
        }

        GenerateAction::UpdateDiffText(text) => {
            session.diff_text = text.clone();

            // Auto-fetch PR context if it looks like a GitHub PR ref
            if crate::infra::github::parse_pr_ref(&text).is_some()
                && !session.is_preview_fetching
                && session.last_preview_input_ref.as_deref() != Some(text.trim())
            {
                session.is_preview_fetching = true;
                session.last_preview_input_ref = Some(text.trim().to_string());
                session.generation_error = None;
                session.generate_preview = None;
                return vec![Command::FetchPrContextPreview {
                    input_ref: text.trim().to_string(),
                }];
            }
            Vec::new()
        }

        GenerateAction::FetchPrContext(input_ref) => {
            let input_ref = input_ref.trim().to_string();
            if input_ref.is_empty() {
                return Vec::new();
            }
            if session.is_preview_fetching {
                return Vec::new();
            }
            if session.last_preview_input_ref.as_deref() == Some(input_ref.as_str())
                && session.generate_preview.is_some()
            {
                return Vec::new();
            }

            session.is_preview_fetching = true;
            session.last_preview_input_ref = Some(input_ref.clone());
            session.generation_error = None;
            session.generate_preview = None;

            vec![Command::FetchPrContextPreview { input_ref }]
        }
        GenerateAction::SelectAgent(agent) => {
            session.selected_agent = agent;
            Vec::new()
        }
        GenerateAction::ClearTimeline => {
            session.reset_agent_timeline();
            Vec::new()
        }
        GenerateAction::SelectRepo(repo_id) => {
            ui.selected_repo_id = repo_id;
            Vec::new()
        }
        GenerateAction::ToggleAgentPanel => {
            ui.agent_panel_collapsed = !ui.agent_panel_collapsed;
            Vec::new()
        }
        GenerateAction::TogglePlanPanel => {
            ui.plan_panel_collapsed = !ui.plan_panel_collapsed;
            Vec::new()
        }
    }
}

pub fn reduce_msg(
    ui: &mut UiState,
    session: &mut SessionState,
    domain: &mut DomainState,
    msg: GenMsg,
) -> Vec<Command> {
    match msg {
        GenMsg::PreviewResolved { input_ref, result } => {
            session.is_preview_fetching = false;
            if session.diff_text.trim() != input_ref.trim() {
                return Vec::new();
            }
            match result {
                Ok(preview) => {
                    session.generate_preview = Some(preview.clone());
                    session.generation_error = None;

                    if let Some(github) = &preview.github {
                        // Try to auto-select repo by matching "owner/repo" in remotes
                        let search_pattern = format!("{}/{}", github.pr.owner, github.pr.repo);
                        if let Some(matched_repo) = domain.linked_repos.iter().find(|r| {
                            r.remotes
                                .iter()
                                .any(|remote: &String| remote.contains(&search_pattern))
                        }) {
                            ui.selected_repo_id = Some(matched_repo.id.clone());
                        }
                    }
                }
                Err(err) => {
                    session.generate_preview = None;
                    session.generation_error = Some(err);
                }
            }
            Vec::new()
        }
        GenMsg::InputResolved(result) => match *result {
            Ok(payload) => {
                session.generate_preview = Some(payload.preview);
                session.generating_review_id = Some(payload.run_context.review_id.clone());
                ui.selected_review_id = Some(payload.run_context.review_id.clone());
                ui.selected_run_id = Some(payload.run_context.run_id.clone());
                vec![Command::StartGeneration {
                    run_context: Box::new(payload.run_context),
                    selected_agent_id: session.selected_agent.id.clone(),
                }]
            }
            Err(err) => {
                session.is_generating = false;
                session.generation_error = Some(err);
                Vec::new()
            }
        },
        GenMsg::Progress(evt) => {
            let mut commands = Vec::new();
            match &*evt {
                crate::infra::acp::ProgressEvent::TaskStarted(_)
                | crate::infra::acp::ProgressEvent::TaskAdded(_)
                | crate::infra::acp::ProgressEvent::CommentAdded
                | crate::infra::acp::ProgressEvent::MetadataUpdated => {
                    commands.push(Command::RefreshReviewData {
                        reason: ReviewDataRefreshReason::Incremental,
                    });
                }
                _ => {}
            }
            session.ingest_progress(*evt);
            commands
        }
        GenMsg::Done(result) => {
            session.is_generating = false;
            session.generating_review_id = None;

            match result {
                Ok(_payload) => vec![Command::RefreshReviewData {
                    reason: ReviewDataRefreshReason::AfterGeneration,
                }],
                Err(err) => {
                    session.generation_error = Some(err);
                    Vec::new()
                }
            }
        }
    }
}
