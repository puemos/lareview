//! Agent discovery module for LaReview
//! Detects and manages available ACP (Agent Client Protocol) agents such as Codex, Qwen, Gemini, and Mistral ACP-Vibe.

use std::sync::Mutex;
use crate::infra::app_config::{load_config, AppConfig};

/// Information about a discoverable ACP agent
#[derive(Debug, Clone, PartialEq)]
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
    extra_path_snapshot: Option<String>,
    config_snapshot: AppConfig,
}

impl AgentCache {}

static AGENT_CACHE: Mutex<Option<AgentCache>> = Mutex::new(None);

const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(30); // 30 seconds

/// Get a list of all known ACP agent candidates with their availability status
/// Uses a cached AgentRegistry to avoid recreating it on each call, with TTL
pub fn list_agent_candidates() -> Vec<AgentCandidate> {
    let mut cache_guard = AGENT_CACHE.lock().unwrap();
    let extra_path_snapshot = std::env::var("LAREVIEW_EXTRA_PATH").ok();
    let config = load_config();

    // Check if cache is valid (not expired and config hasn't changed)
    if let Some(cache) = cache_guard.as_ref()
        && cache.last_updated.elapsed() < CACHE_TTL
        && cache.extra_path_snapshot == extra_path_snapshot
        // Simple heuristic: check if custom agents count or overrides count changed
        && cache.config_snapshot.custom_agents.len() == config.custom_agents.len()
        && cache.config_snapshot.agent_path_overrides.len() == config.agent_path_overrides.len()
        && cache.config_snapshot.agent_envs.len() == config.agent_envs.len()
    {
        return cache.candidates.clone();
    }

    // Cache is expired or doesn't exist, rebuild it
    
    // 1. Apply environment variables from config before discovery
    // Note: This modifies the process environment, which built-in agents read.
    for envs in config.agent_envs.values() {
        for (key, value) in envs {
            // Built-in agents often use AGENT_ID_ACP_BIN etc.
            // We set them globally so built-in agents find them.
            unsafe {
                std::env::set_var(key, value);
            }
        }
    }

    let registry = crate::infra::acp::AgentRegistry::default();
    let mut candidates: Vec<AgentCandidate> = registry
        .get_agents()
        .iter()
        .map(|agent| agent.candidate())
        .collect();

    // 2. Apply path overrides for built-in agents
    for candidate in candidates.iter_mut() {
        if let Some(override_path) = config.agent_path_overrides.get(&candidate.id) {
            candidate.command = Some(override_path.clone());
            candidate.available = !override_path.is_empty();
        }
    }

    // 3. Add custom agents
    for custom in &config.custom_agents {
        let available = !custom.command.is_empty();
        candidates.push(AgentCandidate {
            id: custom.id.clone(),
            label: custom.label.clone(),
            logo: custom.logo.clone(),
            command: Some(custom.command.clone()),
            args: custom.args.clone(),
            available,
        });
    }

    *cache_guard = Some(AgentCache {
        candidates: candidates.clone(),
        last_updated: std::time::Instant::now(),
        extra_path_snapshot,
        config_snapshot: config,
    });

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::app_config::CustomAgentConfig;

    #[test]
    fn test_agent_candidate_resolution() {
        let mut candidate = AgentCandidate {
            id: "test".into(),
            label: "Test".into(),
            logo: None,
            command: Some("ls".into()),
            args: vec![],
            available: false,
        };

        // Should resolve to full path if available
        let path = crate::infra::brew::find_bin("ls").map(|p| p.to_string_lossy().to_string());
        if let Some(p) = path {
            candidate.command = Some(p.clone());
            candidate.available = true;
            assert!(candidate.available);
        }
    }

    #[test]
    fn test_list_agent_candidates() {
        let candidates = list_agent_candidates();
        assert!(!candidates.is_empty());
    }

    #[test]
    fn test_custom_agent_discovery() {
        let mut config = load_config();
        config.custom_agents.push(CustomAgentConfig {
            id: "custom-test".into(),
            label: "Custom Test".into(),
            logo: None,
            command: "echo".into(),
            args: vec!["hello".into()],
        });
        crate::infra::app_config::save_config(&config).unwrap();

        // Invalidate cache by forcing an update (or just waiting, but let's clear it)
        {
            let mut cache_guard = AGENT_CACHE.lock().unwrap();
            *cache_guard = None;
        }

        let candidates = list_agent_candidates();
        let custom = candidates.iter().find(|c| c.id == "custom-test");
        assert!(custom.is_some());
        assert_eq!(custom.unwrap().label, "Custom Test");
        
        // Cleanup
        config.custom_agents.retain(|c| c.id != "custom-test");
        crate::infra::app_config::save_config(&config).unwrap();
    }
}
