//! ACP (Agent Client Protocol) integration

mod agent_discovery;
mod agent_trait;
mod agents;
mod learning_compactor;
mod task_generator;
mod task_mcp_server;

pub use agent_discovery::{AgentCandidate, invalidate_agent_cache, list_agent_candidates};
pub use agents::AgentRegistry;
pub use learning_compactor::{LearningCompactionInput, run_learning_compaction};
pub use task_generator::{
    GenerateTasksInput, ProgressEvent, build_prompt_with_patterns, generate_tasks_with_acp,
};
pub use task_mcp_server::RunContext;
#[allow(unused_imports)]
pub use task_mcp_server::run_task_mcp_server;
