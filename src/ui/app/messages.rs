use crate::infra::acp::ProgressEvent;

#[derive(Debug, Clone)]
pub struct GhStatusPayload {
    pub gh_path: String,
    pub login: Option<String>,
}

#[derive(Debug)]
pub enum GhMsg {
    Done(Result<GhStatusPayload, String>),
}

#[derive(Debug)]
pub struct GenerateResolvedPayload {
    pub run_context: crate::infra::acp::RunContext,
    pub preview: crate::ui::app::state::GeneratePreview,
}

/// Payload we care about from ACP.
#[allow(dead_code)]
#[derive(Debug)]
pub struct GenResultPayload {
    pub messages: Vec<String>,
    pub thoughts: Vec<String>,
    pub logs: Vec<String>,
}

/// Messages coming back from the async generation task.
#[derive(Debug)]
pub enum GenMsg {
    InputResolved(Box<Result<GenerateResolvedPayload, String>>),
    PreviewResolved {
        input_ref: String,
        result: Result<crate::ui::app::state::GeneratePreview, String>,
    },
    Progress(Box<ProgressEvent>),
    Done(Result<GenResultPayload, String>),
}
