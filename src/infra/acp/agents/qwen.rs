//! Qwen ACP agent implementation

use super::super::agent_discovery::AgentCandidate;

// Check if a command is available in PATH
fn is_command_available(command: &str) -> bool {
    which::which(command).is_ok()
}

pub fn qwen_candidate() -> AgentCandidate {
    AgentCandidate {
        id: "qwen".to_string(),
        label: "Qwen Code".to_string(),
        command: Some("qwen".to_string()),
        args: vec!["--experimental-acp".to_string()],
        available: is_command_available("qwen"),
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
