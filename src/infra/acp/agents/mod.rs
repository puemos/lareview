//! ACP agents module - one file per agent

pub mod claude;
pub mod codex;
pub mod gemini;
pub mod kimi;
pub mod mistral;
pub mod opencode;
pub mod qwen;

pub use super::agent_trait::AgentRegistry;

// Re-export the agent trait
