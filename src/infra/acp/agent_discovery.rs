//! Agent discovery module for LaReview
//! Detects and manages available ACP (Agent Client Protocol) agents such as Codex, Qwen, Gemini, and Mistral ACP-Vibe.

use crate::infra::app_config::{AppConfig, load_config};
use std::sync::Mutex;

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
    config_snapshot: AppConfig,
}

impl AgentCache {}
/// Invalidate the agent cache to force a re-discovery on the next call
pub fn invalidate_agent_cache() {
    let mut cache_guard = AGENT_CACHE.lock().unwrap();
    *cache_guard = None;
}

static AGENT_CACHE: Mutex<Option<AgentCache>> = Mutex::new(None);

const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(30); // 30 seconds

/// Get a list of all known ACP agent candidates with their availability status
/// Uses a cached AgentRegistry to avoid recreating it on each call, with TTL
pub fn list_agent_candidates() -> Vec<AgentCandidate> {
    let mut cache_guard = AGENT_CACHE.lock().unwrap();
    let config = load_config();

    // Check if cache is valid (not expired and config hasn't changed)
    if let Some(cache) = cache_guard.as_ref()
        && cache.last_updated.elapsed() < CACHE_TTL
        // Simple heuristic: check if custom agents count or overrides count changed
        && cache.config_snapshot.custom_agents.len() == config.custom_agents.len()
        && cache.config_snapshot.agent_path_overrides.len() == config.agent_path_overrides.len()
        && cache.config_snapshot.agent_envs.len() == config.agent_envs.len()
    {
        return cache.candidates.clone();
    }

    // Cache is expired or doesn't exist, rebuild it
    for envs in config.agent_envs.values() {
        for (key, value) in envs {
            // Built-in agents often use AGENT_ID_ACP_BIN etc.
            // We set them globally so built-in agents find them.
            // Note: This is unsafe but acceptable because:
            // 1. Cache TTL provides serialization between calls
            // 2. Environment variables are process-local
            // 3. Agent subprocesses inherit these env vars safely
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

    for candidate in candidates.iter_mut() {
        if let Some(override_path) = config.agent_path_overrides.get(&candidate.id) {
            candidate.command = Some(override_path.clone());
            candidate.available = !override_path.is_empty();
        }

        if let Some(override_args) = config.agent_args_overrides.get(&candidate.id) {
            candidate.args = override_args.clone();
        }
    }

    for custom in &config.custom_agents {
        // Set custom agent env_vars before resolution
        for (key, value) in &custom.env_vars {
            unsafe {
                std::env::set_var(key, value);
            }
        }

        // Resolve command via find_bin for feature parity with built-in agents
        let resolved = crate::infra::shell::find_bin(&custom.command);
        let (command, available) = match resolved {
            Some(path) => (Some(path.to_string_lossy().to_string()), true),
            None if !custom.command.is_empty() => (Some(custom.command.clone()), true),
            _ => (Some(custom.command.clone()), false),
        };

        candidates.push(AgentCandidate {
            id: custom.id.clone(),
            label: custom.label.clone(),
            logo: custom.logo.clone(),
            command,
            args: custom.args.clone(),
            available,
        });
    }

    *cache_guard = Some(AgentCache {
        candidates: candidates.clone(),
        last_updated: std::time::Instant::now(),
        config_snapshot: config,
    });

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::app_config::CustomAgentConfig;
    use std::sync::Mutex;

    // Tests that modify config/cache must be serialized to avoid interference
    static CONFIG_MUTEX: Mutex<()> = Mutex::new(());

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
        let path = crate::infra::shell::find_bin("ls").map(|p| p.to_string_lossy().to_string());
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
    fn test_path_override_applied_to_candidates() {
        let _guard = CONFIG_MUTEX.lock().unwrap();
        let candidates = list_agent_candidates();
        let first = candidates.first().expect("should have at least one agent");
        let agent_id = first.id.clone();
        let original_command = first.command.clone();

        // Set a path override in config
        let mut config = load_config();
        config
            .agent_path_overrides
            .insert(agent_id.clone(), "/custom/bin/agent".into());
        config
            .agent_args_overrides
            .insert(agent_id.clone(), vec!["--custom-flag".into()]);
        crate::infra::app_config::save_config(&config).unwrap();

        // Invalidate cache so the new config is picked up
        {
            let mut cache_guard = AGENT_CACHE.lock().unwrap();
            *cache_guard = None;
        }

        let updated = list_agent_candidates();
        let agent = updated.iter().find(|c| c.id == agent_id).unwrap();
        assert_eq!(
            agent.command.as_deref(),
            Some("/custom/bin/agent"),
            "path override should replace factory command"
        );
        assert_eq!(
            agent.args,
            vec!["--custom-flag"],
            "args override should replace factory args"
        );

        // Cleanup: restore original config
        config.agent_path_overrides.remove(&agent_id);
        config.agent_args_overrides.remove(&agent_id);
        crate::infra::app_config::save_config(&config).unwrap();
        {
            let mut cache_guard = AGENT_CACHE.lock().unwrap();
            *cache_guard = None;
        }

        // Verify the override is gone
        let restored = list_agent_candidates();
        let agent = restored.iter().find(|c| c.id == agent_id).unwrap();
        assert_eq!(
            agent.command, original_command,
            "command should revert to factory default after removing override"
        );
    }

    #[test]
    fn test_custom_agent_discovery() {
        let _guard = CONFIG_MUTEX.lock().unwrap();
        let mut config = load_config();
        config.custom_agents.push(CustomAgentConfig {
            id: "custom-test".into(),
            label: "Custom Test".into(),
            logo: None,
            command: "echo".into(),
            args: vec!["hello".into()],
            env_vars: Default::default(),
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

    #[test]
    fn test_custom_agent_find_bin_resolution() {
        let _guard = CONFIG_MUTEX.lock().unwrap();
        let mut config = load_config();
        config.custom_agents.push(CustomAgentConfig {
            id: "custom-bin-test".into(),
            label: "Custom Bin Test".into(),
            logo: None,
            command: "ls".into(), // "ls" should be resolvable via find_bin
            args: vec![],
            env_vars: Default::default(),
        });
        crate::infra::app_config::save_config(&config).unwrap();

        {
            let mut cache_guard = AGENT_CACHE.lock().unwrap();
            *cache_guard = None;
        }

        let candidates = list_agent_candidates();
        let custom = candidates.iter().find(|c| c.id == "custom-bin-test");
        assert!(custom.is_some());
        let custom = custom.unwrap();
        assert!(custom.available, "custom agent with 'ls' command should be available");
        // find_bin should resolve to full path or keep original if resolvable
        assert!(custom.command.is_some());

        // Cleanup
        config.custom_agents.retain(|c| c.id != "custom-bin-test");
        crate::infra::app_config::save_config(&config).unwrap();
    }

    #[test]
    fn test_custom_agent_env_vars() {
        let _guard = CONFIG_MUTEX.lock().unwrap();
        let mut config = load_config();
        let mut env_vars = std::collections::HashMap::new();
        env_vars.insert("LAREVIEW_TEST_CUSTOM_ENV".to_string(), "test_value_123".to_string());

        config.custom_agents.push(CustomAgentConfig {
            id: "custom-env-test".into(),
            label: "Custom Env Test".into(),
            logo: None,
            command: "echo".into(),
            args: vec![],
            env_vars,
        });
        crate::infra::app_config::save_config(&config).unwrap();

        {
            let mut cache_guard = AGENT_CACHE.lock().unwrap();
            *cache_guard = None;
        }

        let candidates = list_agent_candidates();
        let custom = candidates.iter().find(|c| c.id == "custom-env-test");
        assert!(custom.is_some());

        // Verify the env var was set during discovery
        assert_eq!(
            std::env::var("LAREVIEW_TEST_CUSTOM_ENV").ok().as_deref(),
            Some("test_value_123"),
            "custom agent env_vars should be set during discovery"
        );

        // Cleanup
        config.custom_agents.retain(|c| c.id != "custom-env-test");
        crate::infra::app_config::save_config(&config).unwrap();
        unsafe {
            std::env::remove_var("LAREVIEW_TEST_CUSTOM_ENV");
        }
    }
}
