//! OpenCode ACP agent implementation

use super::super::agent_discovery::AgentCandidate;

// Check if a command is available in PATH
fn is_command_available(command: &str) -> bool {
    which::which(command).is_ok()
}

/// OpenCode candidate (static for now).
pub fn opencode_candidate() -> AgentCandidate {
    AgentCandidate {
        id: "opencode".to_string(),
        label: "OpenCode".to_string(),
        logo: Some("assets/icons/opencode.svg".to_string()),
        command: Some("opencode".to_string()),
        args: vec!["acp".to_string()],
        available: is_command_available("opencode"),
    }
}

// Implement the ACP Agent trait for opencode agent
pub struct OpenCodeAgent;

impl super::super::agent_trait::AcpAgent for OpenCodeAgent {
    fn id(&self) -> &'static str {
        "opencode"
    }

    fn display_name(&self) -> &'static str {
        "OpenCode"
    }

    fn candidate(&self) -> AgentCandidate {
        opencode_candidate()
    }

    fn is_available(&self) -> bool {
        is_command_available("opencode")
    }
}
