//! Gemini ACP agent implementation

use super::super::agent_discovery::AgentCandidate;

// Check if a command is available in PATH
fn is_command_available(command: &str) -> bool {
    which::which(command).is_ok()
}

/// Gemini candidate (static for now).
pub fn gemini_candidate() -> AgentCandidate {
    AgentCandidate {
        id: "gemini".to_string(),
        label: "Gemini (ACP)".to_string(),
        command: Some("gemini".to_string()),
        args: vec!["--experimental-acp".to_string()],
        available: is_command_available("gemini"),
    }
}

// Implement the ACP Agent trait for gemini agent
pub struct GeminiAgent;

impl super::super::agent_trait::AcpAgent for GeminiAgent {
    fn id(&self) -> &'static str {
        "gemini"
    }

    fn display_name(&self) -> &'static str {
        "Gemini (ACP)"
    }

    fn candidate(&self) -> AgentCandidate {
        gemini_candidate()
    }

    fn is_available(&self) -> bool {
        is_command_available("gemini")
    }
}
