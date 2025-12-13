//! Mistral ACP agent implementation with Vibe functionality

use super::super::agent_discovery::AgentCandidate;

// Check if a command is available in PATH
fn is_command_available(command: &str) -> bool {
    which::which(command).is_ok()
}

/// Mistral ACP-Vibe candidate
pub fn mistral_candidate() -> AgentCandidate {
    AgentCandidate {
        id: "mistral".to_string(),
        label: "Mistral".to_string(),
        logo: Some("assets/icons/mistral.svg".to_string()),
        command: Some("vibe-acp".to_string()),
        args: vec![],
        available: is_command_available("vibe-acp"),
    }
}

// Implement the ACP Agent trait for mistral agent
pub struct MistralAgent;

impl super::super::agent_trait::AcpAgent for MistralAgent {
    fn id(&self) -> &'static str {
        "mistral"
    }

    fn display_name(&self) -> &'static str {
        "Mistral"
    }

    fn candidate(&self) -> AgentCandidate {
        mistral_candidate()
    }

    fn is_available(&self) -> bool {
        is_command_available("vibe-acp")
    }
}
