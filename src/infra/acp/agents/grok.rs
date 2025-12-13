//! Grok ACP agent implementation

use super::super::agent_discovery::AgentCandidate;

// Check if a command is available in PATH
fn is_command_available(command: &str) -> bool {
    which::which(command).is_ok()
}

/// Grok candidate (static for now).
pub fn grok_candidate() -> AgentCandidate {
    AgentCandidate {
        id: "grok".to_string(),
        label: "Grok".to_string(),
        logo: Some("assets/icons/grok.svg".to_string()),
        command: Some("grok".to_string()),
        args: vec!["--experimental-acp".to_string()],
        available: is_command_available("grok"),
    }
}

// Implement the ACP Agent trait for grok agent
pub struct GrokAgent;

impl super::super::agent_trait::AcpAgent for GrokAgent {
    fn id(&self) -> &'static str {
        "grok"
    }

    fn display_name(&self) -> &'static str {
        "Grok"
    }

    fn candidate(&self) -> AgentCandidate {
        grok_candidate()
    }

    fn is_available(&self) -> bool {
        is_command_available("grok")
    }
}
