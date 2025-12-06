//! ACP (Agent Client Protocol) integration

mod agent_discovery;
mod task_generator;
mod task_mcp_server;

pub use agent_discovery::list_agent_candidates;
pub use task_generator::{GenerateTasksInput, ProgressEvent, generate_tasks_with_acp};
pub use task_mcp_server::run_task_mcp_server;
