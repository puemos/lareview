//! Minimal MCP server that exposes a single `return_tasks` tool.
//!
//! The tool expects a JSON object `{ "tasks": [...] }` and writes it verbatim to the path
//! specified by the `TASK_MCP_OUT` environment variable. The server runs over stdio so the ACP
//! agent can launch it as an MCP server.

mod config;
mod logging;
mod parsing;
mod persistence;
mod tool;
mod transport;

pub use config::ServerConfig;
pub(crate) use parsing::parse_tasks;

use std::sync::Arc;

use pmcp::{Server, ServerCapabilities};

/// Run the MCP server over stdio. Blocks until the process is terminated.
pub async fn run_task_mcp_server() -> pmcp::Result<()> {
    let config = Arc::new(ServerConfig::from_args());
    logging::log_to_file(&config, "starting task MCP server");

    let server = Server::builder()
        .name("lareview-tasks")
        .version("0.1.0")
        .capabilities(ServerCapabilities::default())
        .tool(
            "return_tasks",
            tool::create_return_tasks_tool(config.clone()),
        )
        .build()?;

    logging::log_to_file(&config, "running task MCP server on stdio (line-delimited)");
    let transport = transport::LineDelimitedStdioTransport::new();
    server.run(transport).await
}

#[cfg(test)]
mod tests;
