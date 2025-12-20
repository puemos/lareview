//! Kimi CLI ACP agent implementation

use super::super::agent_discovery::AgentCandidate;

// Check if a command is available in PATH
fn is_command_available(command: &str) -> bool {
    crate::infra::brew::find_bin(command).is_some()
}

/// Kimi candidate (static for now).
pub fn kimi_candidate() -> AgentCandidate {
    let command =
        crate::infra::brew::find_bin("kimi").map(|path| path.to_string_lossy().to_string());
    let available = command.is_some();

    AgentCandidate {
        id: "kimi".to_string(),
        label: "Kimi CLI".to_string(),
        logo: Some("assets/icons/kimi.svg".to_string()),
        command,
        args: vec!["--acp".to_string()],
        available,
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
