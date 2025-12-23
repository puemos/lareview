use crate::infra::acp::task_mcp_server::RunContext;
use agent_client_protocol::SessionUpdate;
use std::path::PathBuf;

/// Input parameters for task generation.
pub struct GenerateTasksInput {
    /// Review/run context for the task generation (also persisted by the MCP server).
    pub run_context: RunContext,
    /// Optional repository root for read-only context.
    ///
    /// When this is None, the agent must operate diff-only without filesystem or terminal access.
    /// When Some, the agent may read files under this root for context only.
    pub repo_root: Option<PathBuf>,
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
    /// Raw ACP session update, streamed to the UI.
    Update(Box<SessionUpdate>),
    /// Local log output from the ACP worker/process.
    LocalLog(String),
    /// Signal that the agent has finished its work (received finalize_review).
    Finalized,
    /// A new task is being generated.
    TaskStarted(String),
    /// A new task has been successfully persisted by the MCP server.
    TaskAdded(String),
    /// A new comment has been successfully persisted by the MCP server.
    CommentAdded,
    /// Review metadata (title/summary) has been updated by the MCP server.
    MetadataUpdated,
}
