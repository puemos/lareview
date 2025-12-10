//! Qwen ACP agent implementation

use crate::acp::agent_discovery::AgentCandidate;
use std::env;
use std::process::Command;

// Check if a command is available in PATH
fn is_command_available(command: &str) -> bool {
    which::which(command).is_ok()
}

/// Ensure Qwen MCP is registered
pub fn ensure_qwen_mcp_registered() {
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

    // Register for Qwen MCP
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

pub fn qwen_candidate() -> AgentCandidate {
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

// Implement the ACP Agent trait for qwen agent
pub struct QwenAgent;

impl super::super::agent_trait::AcpAgent for QwenAgent {
    fn id(&self) -> &'static str {
        "qwen"
    }

    fn display_name(&self) -> &'static str {
        "Qwen Code (ACP)"
    }

    fn candidate(&self) -> AgentCandidate {
        qwen_candidate()
    }

    fn is_available(&self) -> bool {
        // Make sure our MCP is installed once
        ensure_qwen_mcp_registered();
        is_command_available("qwen")
    }
}
