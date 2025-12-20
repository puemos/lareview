//! OpenCode ACP agent implementation

use super::super::agent_discovery::AgentCandidate;

// Check if a command is available in PATH
fn is_command_available(command: &str) -> bool {
    crate::infra::brew::find_bin(command).is_some()
}

/// OpenCode candidate (static for now).
pub fn opencode_candidate() -> AgentCandidate {
    let command =
        crate::infra::brew::find_bin("opencode").map(|path| path.to_string_lossy().to_string());
    let available = command.is_some();

    AgentCandidate {
        id: "opencode".to_string(),
        label: "OpenCode".to_string(),
        logo: Some("assets/icons/opencode.svg".to_string()),
        command,
        args: vec!["acp".to_string()],
        available,
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
