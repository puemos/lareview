//! ACP (Agent Client Protocol) integration

mod agent_discovery;
mod diff_parser;
mod task_generator;

pub use agent_discovery::{list_agent_candidates, AgentCandidate};
pub use diff_parser::parse_diff;
pub use task_generator::{generate_tasks_with_acp, GenerateTasksInput, GenerateTasksResult};
