//! Codex ACP agent implementation

use super::super::agent_discovery::AgentCandidate;

// Check if a command is available in PATH
fn is_command_available(command: &str) -> bool {
    crate::infra::brew::find_bin(command).is_some()
}

/// Build the Codex candidate, allowing overrides for binary/package.
pub fn codex_candidate() -> AgentCandidate {
    let bin_override = std::env::var("CODEX_ACP_BIN").ok();
    let command = match bin_override.as_deref() {
        Some(bin) => crate::infra::brew::find_bin(bin),
        None => crate::infra::brew::find_bin("npx"),
    };
    let available = command.is_some();
    let args = if bin_override.is_some() {
        Vec::new()
    } else {
        vec![
            "-y".to_string(),
            "@zed-industries/codex-acp@latest".to_string(),
            "-c".to_string(),
            "model=\"gpt-5.2\"".to_string(),
        ]
    };

    AgentCandidate {
        id: "codex".to_string(),
        label: "Codex".to_string(),
        logo: Some("assets/icons/codex.svg".to_string()),
        command: command.map(|path| path.to_string_lossy().to_string()),
        args,
        available,
    }
}

// Implement the ACP Agent trait for codex agent
pub struct CodexAgent;

impl super::super::agent_trait::AcpAgent for CodexAgent {
    fn id(&self) -> &'static str {
        "codex"
    }

    fn display_name(&self) -> &'static str {
        "Codex"
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::acp::agent_trait::AcpAgent;

    #[test]
    fn test_codex_agent_basics() {
        let agent = CodexAgent;
        assert_eq!(agent.id(), "codex");
        assert_eq!(agent.display_name(), "Codex");
        let _ = agent.candidate();
        let _ = agent.is_available();
    }

    #[test]
    fn test_codex_agent_available_override() {
        unsafe {
            std::env::set_var("CODEX_ACP_BIN", "ls");
        }
        let agent = CodexAgent;
        assert!(agent.is_available());
        unsafe {
            std::env::remove_var("CODEX_ACP_BIN");
        }
    }
}
