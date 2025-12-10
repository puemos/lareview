//! Codex ACP agent implementation

use crate::acp::agent_discovery::AgentCandidate;

// Check if a command is available in PATH
fn is_command_available(command: &str) -> bool {
    which::which(command).is_ok()
}

/// Build the Codex candidate, allowing overrides for binary/package.
pub fn codex_candidate() -> AgentCandidate {
    // Allow overrides for package/bin; do not inject partial MCP config via -c.
    let package = std::env::var("CODEX_ACP_PACKAGE")
        .unwrap_or_else(|_| "@zed-industries/codex-acp@latest".to_string());

    if let Ok(bin_path) = std::env::var("CODEX_ACP_BIN") {
        AgentCandidate {
            id: "codex".to_string(),
            label: "Codex (ACP)".to_string(),
            command: Some(bin_path.clone()),
            args: Vec::new(),
            available: is_command_available(&bin_path),
        }
    } else {
        AgentCandidate {
            id: "codex".to_string(),
            label: "Codex (ACP)".to_string(),
            command: Some("npx".to_string()),
            args: vec!["-y".to_string(), package],
            available: is_command_available("npx"),
        }
    }
}

// Implement the ACP Agent trait for codex agent
pub struct CodexAgent;

impl super::super::agent_trait::AcpAgent for CodexAgent {
    fn id(&self) -> &'static str {
        "codex"
    }

    fn display_name(&self) -> &'static str {
        "Codex (ACP)"
    }

    fn candidate(&self) -> AgentCandidate {
        codex_candidate()
    }

    fn is_available(&self) -> bool {
        let _package = std::env::var("CODEX_ACP_PACKAGE")
            .unwrap_or_else(|_| "@zed-industries/codex-acp@latest".to_string());

        if let Ok(bin_path) = std::env::var("CODEX_ACP_BIN") {
            is_command_available(&bin_path)
        } else {
            is_command_available("npx")
        }
    }
}
