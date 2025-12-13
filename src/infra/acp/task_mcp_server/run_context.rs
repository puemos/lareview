use crate::domain::ReviewSource;

/// Context provided by the UI/runtime to the MCP server so it can persist review output.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RunContext {
    pub review_id: String,
    pub run_id: String,
    pub agent_id: String,
    pub input_ref: String,
    pub diff_text: String,
    pub diff_hash: String,
    pub source: ReviewSource,
    #[serde(default)]
    pub initial_title: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}
