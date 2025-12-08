//! Agent discovery module for LaReview
//! Detects and manages available ACP (Agent Client Protocol) agents such as Codex, Qwen, and Gemini.

/// Information about a discoverable ACP agent
#[derive(Debug, Clone)]
pub struct AgentCandidate {
    /// Unique identifier for the agent (e.g., "codex", "qwen", "gemini")
    pub id: String,
    /// User-friendly display name for the agent
    pub label: String,
    /// Command to execute the agent, if available
    pub command: Option<String>,
    /// Arguments to pass to the agent command
    pub args: Vec<String>,
    /// Whether the agent is available and can be executed
    pub available: bool,
}

fn ensure_qwen_mcp_registered() {
    use std::env;
    use std::process::Command;

    // Full path of this executable
    let exe = env::current_exe().unwrap_or_else(|_| "lareview".into());
    let exe_str = exe.to_string_lossy().to_string();

    // Check if already exists
    let check = Command::new("qwen").args(["mcp", "list"]).output();

    if let Ok(out) = check {
        let stdout = String::from_utf8_lossy(&out.stdout);
        if stdout.contains("lareview-tasks") {
            return;
        }
    }

    // Register for Gemini MCP
    let _ = Command::new("qwen")
        .args([
            "mcp",
            "add",
            "lareview-tasks",
            &exe_str,
            "--task-mcp-server",
        ])
        .status();
}

fn qwen_candidate() -> AgentCandidate {
    // make sure our MCP is installed once
    ensure_qwen_mcp_registered();

    AgentCandidate {
        id: "qwen".to_string(),
        label: "Qwen Code (ACP)".to_string(),
        command: Some("qwen".to_string()),
        args: vec!["--experimental-acp".to_string()],
        available: is_command_available("qwen"),
    }
}

/// Build the Codex candidate, allowing overrides for binary/package.
fn codex_candidate() -> AgentCandidate {
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

/// Gemini candidate (static for now).
fn gemini_candidate() -> AgentCandidate {
    AgentCandidate {
        id: "gemini".to_string(),
        label: "Gemini (ACP)".to_string(),
        command: Some("gemini".to_string()),
        args: vec!["--experimental-acp".to_string()],
        available: is_command_available("gemini"),
    }
}

/// Check if a command is available in PATH
fn is_command_available(command: &str) -> bool {
    which::which(command).is_ok()
}

/// Get a list of all known ACP agent candidates with their availability status
/// Includes built-in stub, Codex, Gemini, and Qwen agents
pub fn list_agent_candidates() -> Vec<AgentCandidate> {
    let mut candidates = vec![AgentCandidate {
        id: "stub".to_string(),
        label: "Built-in stub".to_string(),
        command: None,
        args: Vec::new(),
        available: true,
    }];

    candidates.push(codex_candidate());
    candidates.push(gemini_candidate());
    candidates.push(qwen_candidate());

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lists_known_agents() {
        let candidates = list_agent_candidates();
        assert!(candidates.iter().any(|c| c.id == "codex"));
        assert!(candidates.iter().any(|c| c.id == "gemini"));
    }
}
