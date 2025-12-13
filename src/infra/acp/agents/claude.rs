//! Claude ACP agent implementation

use super::super::agent_discovery::AgentCandidate;

// Check if a command is available in PATH
fn is_command_available(command: &str) -> bool {
    which::which(command).is_ok()
}

/// Claude candidate (static for now).
pub fn claude_candidate() -> AgentCandidate {
    AgentCandidate {
        id: "claude".to_string(),
        label: "Claude".to_string(),
        logo: Some("assets/icons/claude.svg".to_string()),
        command: Some("claude".to_string()),
        args: vec!["--experimental-acp".to_string()],
        available: is_command_available("claude"),
    }
}

// Implement the ACP Agent trait for claude agent
pub struct ClaudeAgent;

impl super::super::agent_trait::AcpAgent for ClaudeAgent {
    fn id(&self) -> &'static str {
        "claude"
    }

    fn display_name(&self) -> &'static str {
        "Claude"
    }

    fn candidate(&self) -> AgentCandidate {
        claude_candidate()
    }

    fn is_available(&self) -> bool {
        is_command_available("claude")
    }
}
