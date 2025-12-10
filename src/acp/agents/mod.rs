//! ACP agents module - one file per agent

pub mod codex;
pub mod gemini;
pub mod mistral;
pub mod qwen;

pub use crate::acp::agent_trait::AgentRegistry;

// Re-export the agent trait
