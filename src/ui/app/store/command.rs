use crate::domain::PullRequest;

#[derive(Debug, Clone)]
pub enum Command {
    StartGeneration {
        pull_request: Box<PullRequest>,
        diff_text: String,
        selected_agent_id: String,
    },
    SyncReviewAfterGeneration,
}
