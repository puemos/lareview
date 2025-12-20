//! Minimal MCP server that exposes streaming `return_task` and `finalize_review` tools.
//!
//! The server accepts individual tasks via `return_task` then finalizes with `finalize_review`.
//! It also supports legacy bulk tools for backward compatibility.
//! The server runs over stdio so the ACP agent can launch it as an MCP server.

mod config;
mod logging;
mod parsing;
mod persistence;
mod run_context;
mod task_ingest;
mod tool;
mod transport;

pub use config::ServerConfig;
pub(crate) use parsing::parse_task;
pub use run_context::RunContext;

use std::sync::Arc;

use pmcp::{Server, ServerCapabilities};

/// Run the MCP server over stdio. Blocks until the process is terminated.
pub async fn run_task_mcp_server() -> pmcp::Result<()> {
    let config = Arc::new(ServerConfig::from_args());
    logging::log_to_file(&config, "starting task MCP server");

    let server = Server::builder()
        .name("lareview-tasks")
        .version(env!("CARGO_PKG_VERSION"))
        .capabilities(ServerCapabilities::tools_only())
        // New streaming tools
        .tool("return_task", tool::create_return_task_tool(config.clone()))
        .tool("repo_search", tool::create_repo_search_tool(config.clone()))
        .tool(
            "repo_list_files",
            tool::create_repo_list_files_tool(config.clone()),
        )
        .tool(
            "finalize_review",
            tool::create_finalize_review_tool(config.clone()),
        )
        .build()?;

    logging::log_to_file(&config, "running task MCP server on stdio (line-delimited)");
    let transport = transport::LineDelimitedStdioTransport::new();
    server.run(transport).await
}

#[cfg(test)]
mod tests;
