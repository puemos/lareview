//! Qwen ACP agent implementation

use super::super::agent_discovery::AgentCandidate;

// Check if a command is available in PATH
fn is_command_available(command: &str) -> bool {
    crate::infra::brew::find_bin(command).is_some()
}

pub fn qwen_candidate() -> AgentCandidate {
    let command =
        crate::infra::brew::find_bin("qwen").map(|path| path.to_string_lossy().to_string());
    let available = command.is_some();

    AgentCandidate {
        id: "qwen".to_string(),
        label: "Qwen".to_string(),
        logo: Some("assets/icons/qwen.svg".to_string()),
        command,
        args: vec!["--experimental-acp".to_string()],
        available,
    }
}

// Implement the ACP Agent trait for qwen agent
pub struct QwenAgent;

impl super::super::agent_trait::AcpAgent for QwenAgent {
    fn id(&self) -> &'static str {
        "qwen"
    }

    fn display_name(&self) -> &'static str {
        "Qwen Code"
    }

    fn candidate(&self) -> AgentCandidate {
        qwen_candidate()
    }

    fn is_available(&self) -> bool {
        is_command_available("qwen")
    }
}
