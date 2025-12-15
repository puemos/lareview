//! Kimi CLI ACP agent implementation

use super::super::agent_discovery::AgentCandidate;

// Check if a command is available in PATH
fn is_command_available(command: &str) -> bool {
    which::which(command).is_ok()
}

/// Kimi candidate (static for now).
pub fn kimi_candidate() -> AgentCandidate {
    AgentCandidate {
        id: "kimi".to_string(),
        label: "Kimi CLI".to_string(),
        logo: Some("assets/icons/kimi.svg".to_string()),
        command: Some("kimi".to_string()),
        args: vec!["--acp".to_string()],
        available: is_command_available("kimi"),
    }
}

// Implement the ACP Agent trait for kimi agent
pub struct KimiAgent;

impl super::super::agent_trait::AcpAgent for KimiAgent {
    fn id(&self) -> &'static str {
        "kimi"
    }

    fn display_name(&self) -> &'static str {
        "Kimi CLI"
    }

    fn candidate(&self) -> AgentCandidate {
        kimi_candidate()
    }

    fn is_available(&self) -> bool {
        is_command_available("kimi")
    }
}
