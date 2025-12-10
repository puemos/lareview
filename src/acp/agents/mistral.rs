//! Mistral ACP agent implementation with Vibe functionality

use crate::acp::agent_discovery::AgentCandidate;
use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

// Check if a command is available in PATH
fn is_command_available(command: &str) -> bool {
    which::which(command).is_ok()
}

/// Configuration for an MCP server in Vibe
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum VibeMcpServer {
    /// HTTP transport server
    Http {
        /// A short alias for the server (used in tool names)
        name: String,
        /// The transport type
        transport: String,
        /// Base URL for HTTP transports
        url: String,
        /// Additional HTTP headers
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<std::collections::HashMap<String, String>>,
        /// Environment variable containing the API key
        #[serde(skip_serializing_if = "Option::is_none")]
        api_key_env: Option<String>,
        /// HTTP header name for API key
        #[serde(skip_serializing_if = "Option::is_none")]
        api_key_header: Option<String>,
        /// Format string for API key (e.g., "Bearer {token}")
        #[serde(skip_serializing_if = "Option::is_none")]
        api_key_format: Option<String>,
    },
    /// Stdio transport server
    Stdio {
        /// A short alias for the server (used in tool names)
        name: String,
        /// The transport type
        transport: String,
        /// Command to run for stdio transport
        command: String,
        /// Additional arguments for stdio transport
        args: Vec<String>,
    },
}

/// Vibe configuration structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VibeConfig {
    /// MCP servers configured for Vibe
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<Vec<VibeMcpServer>>,
    // Other Vibe config fields would go here, but we only care about mcp_servers
}

/// Get the path to the Vibe configuration file
fn get_vibe_config_path() -> PathBuf {
    if let Ok(vibe_home) = env::var("VIBE_HOME") {
        PathBuf::from(vibe_home).join("config.toml")
    } else if let Some(home_dir) = home::home_dir() {
        home_dir.join(".vibe").join("config.toml")
    } else {
        PathBuf::from(".vibe").join("config.toml")
    }
}

/// Read the existing Vibe configuration file
fn read_vibe_config() -> Result<VibeConfig> {
    let config_path = get_vibe_config_path();

    if !config_path.exists() {
        return Ok(VibeConfig { mcp_servers: None });
    }

    let config_content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read Vibe config at {}", config_path.display()))?;

    toml::from_str(&config_content).with_context(|| "Failed to parse Vibe config TOML")
}

/// Write the Vibe configuration file
fn write_vibe_config(config: &VibeConfig) -> Result<()> {
    let config_path = get_vibe_config_path();

    // Ensure the directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    let toml_content =
        toml::to_string(config).with_context(|| "Failed to serialize Vibe config to TOML")?;

    fs::write(&config_path, toml_content)
        .with_context(|| format!("Failed to write Vibe config to {}", config_path.display()))
}

/// Add or update the LaReview MCP server in the Vibe configuration
pub fn ensure_vibe_mcp_server_registered() -> Result<()> {
    let mut config = read_vibe_config()?;

    let exe = env::current_exe().unwrap_or_else(|_| "lareview".into());
    let exe_str = exe.to_string_lossy().to_string();

    let lareview_server = VibeMcpServer::Stdio {
        name: "lareview-tasks".to_string(),
        transport: "stdio".to_string(),
        command: exe_str,
        args: vec!["--task-mcp-server".to_string()],
    };

    // Check if the server already exists
    let server_exists = config.mcp_servers.as_ref().is_some_and(|servers| {
        servers.iter().any(|s| match s {
            VibeMcpServer::Http { name, .. } => name == "lareview-tasks",
            VibeMcpServer::Stdio { name, .. } => name == "lareview-tasks",
        })
    });

    if !server_exists {
        match &mut config.mcp_servers {
            Some(servers) => {
                // Remove any existing lareview-tasks server and add the new one
                servers.retain(|s| match s {
                    VibeMcpServer::Http { name, .. } => name != "lareview-tasks",
                    VibeMcpServer::Stdio { name, .. } => name != "lareview-tasks",
                });
                servers.push(lareview_server);
            }
            None => {
                config.mcp_servers = Some(vec![lareview_server]);
            }
        }

        write_vibe_config(&config)?;
        log::info!("Added LaReview MCP server to Vibe configuration");
    } else {
        log::info!("LaReview MCP server already registered in Vibe configuration");
    }

    Ok(())
}

/// Mistral ACP-Vibe candidate
pub fn mistral_vibe_candidate() -> AgentCandidate {
    // Try to register our MCP server with Vibe
    let _ = ensure_vibe_mcp_server_registered();

    AgentCandidate {
        id: "mistral".to_string(),
        label: "Mistral Vibe".to_string(),
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
        "Mistral Vibe"
    }

    fn candidate(&self) -> AgentCandidate {
        mistral_vibe_candidate()
    }

    fn is_available(&self) -> bool {
        // Try to register our MCP server with Vibe
        let _ = ensure_vibe_mcp_server_registered();
        is_command_available("vibe-acp")
    }
}
