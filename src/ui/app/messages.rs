use crate::infra::acp::ProgressEvent;

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
    Progress(Box<ProgressEvent>),
    Done(Result<GenResultPayload, String>),
}
