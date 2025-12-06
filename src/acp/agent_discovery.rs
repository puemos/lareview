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
    pub description: Option<String>,
}

/// Known ACP agent candidates
const CANDIDATES: &[(&str, &str, &str, &[&str], &str)] = &[
    (
        "codex",
        "Codex (ACP)",
        "npx",
        &["-y", "@zed-industries/codex-acp@latest"],
        "Uses the Codex CLI via npx (pre-accept install with -y)",
    ),
    (
        "gemini",
        "Gemini (ACP)",
        "gemini",
        &["--experimental-acp"],
        "Uses the Gemini CLI with --experimental-acp",
    ),
];

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
        description: Some("Generates one task per file locally".to_string()),
    }];

    for (id, label, command, args, description) in CANDIDATES {
        candidates.push(AgentCandidate {
            id: id.to_string(),
            label: label.to_string(),
            command: Some(command.to_string()),
            args: args.iter().map(|s| s.to_string()).collect(),
            available: is_command_available(command),
            description: Some(description.to_string()),
        });
    }

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
