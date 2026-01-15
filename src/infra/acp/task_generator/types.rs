use crate::domain::ResolvedRule;
use crate::infra::acp::task_mcp_server::RunContext;
use std::path::PathBuf;

/// Input parameters for task generation.
pub struct GenerateTasksInput {
    /// Review/run context for the task generation (also persisted by the MCP server).
    pub run_context: RunContext,
    /// Review rules that apply to this run and should be injected into the prompt.
    pub rules: Vec<ResolvedRule>,
    /// Optional repository root for read-only context.
    ///
    /// When this is None, the agent must operate diff-only without filesystem or terminal access.
    /// When Some, the agent may read files under this root for context only.
    pub repo_root: Option<PathBuf>,
    /// Optional snapshot path to cleanup after generation.
    pub cleanup_path: Option<PathBuf>,
    /// Command to execute the ACP agent.
    pub agent_command: String,
    /// Arguments to pass to the ACP agent command.
    pub agent_args: Vec<String>,
    /// Optional channel to send progress updates during generation.
    pub progress_tx: Option<tokio::sync::mpsc::UnboundedSender<ProgressEvent>>,
    /// Override for MCP server binary path.
    pub mcp_server_binary: Option<PathBuf>,
    /// Timeout in seconds for agent execution.
    pub timeout_secs: Option<u64>,
    /// Optional cancellation token to stop the agent.
    pub cancel_token: Option<tokio_util::sync::CancellationToken>,
    /// Enable debug logging.
    pub debug: bool,
}

/// Result of task generation.
#[derive(Debug)]
pub struct GenerateTasksResult {
    pub messages: Vec<String>,
    pub thoughts: Vec<String>,
    pub logs: Vec<String>,
}

/// Different types of progress updates that can be streamed from the agent.
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// Delta for agent message text (only new characters since last update).
    MessageDelta { id: String, delta: String },
    /// Delta for agent thought text (only new characters since last update).
    ThoughtDelta { id: String, delta: String },
    /// Tool call started (phase 1 of two-phase update).
    ToolCallStarted {
        tool_call_id: String,
        title: String,
        kind: String,
    },
    /// Tool call completed with full data (phase 2 of two-phase update).
    ToolCallComplete {
        tool_call_id: String,
        status: String,
        title: String,
        raw_input: Option<serde_json::Value>,
        raw_output: Option<serde_json::Value>,
    },
    /// Plan update (sent as complete object).
    Plan(agent_client_protocol::Plan),
    /// Local log output from the ACP worker/process.
    LocalLog(String),
    /// Signal that the agent has finished its work (received finalize_review).
    Finalized,
    /// A new task is being generated.
    TaskStarted(String, String),
    /// A new task has been successfully persisted by the MCP server.
    TaskAdded(String),
    /// A new comment has been successfully persisted by the MCP server.
    CommentAdded,
    /// Review metadata (title/summary) has been updated by the MCP server.
    MetadataUpdated,
}
