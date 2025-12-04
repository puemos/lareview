//! ACP (Agent Client Protocol) integration

mod agent_discovery;
mod diff_parser;
mod task_generator;

pub use agent_discovery::list_agent_candidates;
pub use task_generator::{GenerateTasksInput, generate_tasks_with_acp};
