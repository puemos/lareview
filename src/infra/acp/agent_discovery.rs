//! Agent discovery module for LaReview
//! Detects and manages available ACP (Agent Client Protocol) agents such as Codex, Qwen, Gemini, and Mistral ACP-Vibe.

use std::sync::Mutex;

/// Information about a discoverable ACP agent
#[derive(Debug, Clone)]
pub struct AgentCandidate {
    /// Unique identifier for the agent (e.g., "codex", "qwen", "gemini")
    pub id: String,
    /// User-friendly display name for the agent
    pub label: String,
    /// Path to the agent's logo
    pub logo: Option<String>,
    /// Command to execute the agent, if available
    pub command: Option<String>,
    /// Arguments to pass to the agent command
    pub args: Vec<String>,
    /// Whether the agent is available and can be executed
    pub available: bool,
}

// Cache for agent candidates with timestamp to allow refresh
struct AgentCache {
    candidates: Vec<AgentCandidate>,
    last_updated: std::time::Instant,
}

impl AgentCache {}

static AGENT_CACHE: Mutex<Option<AgentCache>> = Mutex::new(None);

const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(30); // 30 seconds

/// Get a list of all known ACP agent candidates with their availability status
/// Uses a cached AgentRegistry to avoid recreating it on each call, with TTL
pub fn list_agent_candidates() -> Vec<AgentCandidate> {
    let mut cache_guard = AGENT_CACHE.lock().unwrap();

    // Check if cache is valid (not expired)
    match cache_guard.as_ref() {
        Some(cache) if cache.last_updated.elapsed() < CACHE_TTL => {
            // Return cached data
            cache.candidates.clone()
        }
        _ => {
            // Cache is expired or doesn't exist, rebuild it
            let registry = crate::infra::acp::AgentRegistry::default();
            let candidates: Vec<AgentCandidate> = registry
                .get_agents()
                .iter()
                .map(|agent| agent.candidate())
                .collect();

            *cache_guard = Some(AgentCache {
                candidates: candidates.clone(),
                last_updated: std::time::Instant::now(),
            });

            candidates
        }
    }
}
