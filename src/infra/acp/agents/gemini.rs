//! Gemini ACP agent implementation

use super::super::agent_discovery::AgentCandidate;

// Check if a command is available in PATH
fn is_command_available(command: &str) -> bool {
    crate::infra::brew::find_bin(command).is_some()
}

/// Gemini candidate (static for now).
pub fn gemini_candidate() -> AgentCandidate {
    let command = crate::infra::brew::find_bin("gemini")
        .map(|path| path.to_string_lossy().to_string());
    let available = command.is_some();

    AgentCandidate {
        id: "gemini".to_string(),
        label: "Gemini".to_string(),
        logo: Some("assets/icons/gemini.svg".to_string()),
        command,
        args: vec!["--experimental-acp".to_string()],
        available,
    }
}

// Implement the ACP Agent trait for gemini agent
pub struct GeminiAgent;

impl super::super::agent_trait::AcpAgent for GeminiAgent {
    fn id(&self) -> &'static str {
        "gemini"
    }

    fn display_name(&self) -> &'static str {
        "Gemini"
    }

    fn candidate(&self) -> AgentCandidate {
        gemini_candidate()
    }

    fn is_available(&self) -> bool {
        is_command_available("gemini")
    }
}
