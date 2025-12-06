#![allow(dead_code)]
//! Agent discovery - detect available ACP agents

/// Candidate ACP agent
#[derive(Debug, Clone)]
pub struct AgentCandidate {
    pub id: String,
    pub label: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub available: bool,
}

/// Build the Codex candidate, allowing overrides for binary/package.
fn codex_candidate() -> AgentCandidate {
    // Allow overrides for package/bin; do not inject partial MCP config via -c.
    let package = std::env::var("CODEX_ACP_PACKAGE")
        .unwrap_or_else(|_| "@zed-industries/codex-acp@latest".to_string());

    if let Ok(bin_path) = std::env::var("CODEX_ACP_BIN") {
        AgentCandidate {
            id: "codex".to_string(),
            label: "Codex (ACP)".to_string(),
            command: Some(bin_path.clone()),
            args: Vec::new(),
            available: is_command_available(&bin_path),
        }
    } else {
        AgentCandidate {
            id: "codex".to_string(),
            label: "Codex (ACP)".to_string(),
            command: Some("npx".to_string()),
            args: vec!["-y".to_string(), package],
            available: is_command_available("npx"),
        }
    }
}

/// Gemini candidate (static for now).
fn gemini_candidate() -> AgentCandidate {
    AgentCandidate {
        id: "gemini".to_string(),
        label: "Gemini (ACP)".to_string(),
        command: Some("gemini".to_string()),
        args: vec!["--experimental-acp".to_string()],
        available: is_command_available("gemini"),
    }
}

/// Check if a command is available in PATH
fn is_command_available(command: &str) -> bool {
    which::which(command).is_ok()
}

/// List all known agent candidates with availability status
pub fn list_agent_candidates() -> Vec<AgentCandidate> {
    let mut candidates = vec![AgentCandidate {
        id: "stub".to_string(),
        label: "Built-in stub".to_string(),
        command: None,
        args: Vec::new(),
        available: true,
    }];

    candidates.push(codex_candidate());
    candidates.push(gemini_candidate());

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lists_known_agents() {
        let candidates = list_agent_candidates();
        assert!(candidates.iter().any(|c| c.id == "codex"));
        assert!(candidates.iter().any(|c| c.id == "gemini"));
    }
}
