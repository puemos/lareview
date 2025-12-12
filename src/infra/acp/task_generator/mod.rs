//! Task generator module for LaReview.
//!
//! Handles communication with ACP (Agent Client Protocol) agents to generate
//! review tasks from git diffs using AI agents like Codex, Qwen, and Gemini.

mod client;
mod prompt;
mod types;
mod validation;
mod worker;

pub use types::{GenerateTasksInput, GenerateTasksResult, ProgressEvent};
pub use worker::generate_tasks_with_acp;

#[cfg(test)]
mod mcp_config_tests;

#[cfg(test)]
mod persistence_tests;

#[cfg(test)]
mod policy_tests;

#[cfg(test)]
mod real_acp_tests;
