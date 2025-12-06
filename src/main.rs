//! LaReview - Intent-based Pull Request Review Tool
//!
//! A native desktop application built with GPUI for reviewing
//! pull requests by intent, not by file order.

mod acp;
mod data;
mod domain;
mod ui;

use gpui::{App, AppContext, Application, WindowOptions};
use std::env;
use std::process;
use ui::app::LaReviewApp;

fn main() {
    // Special mode: run the MCP task server and exit.
    if env::args().any(|arg| arg == "--task-mcp-server") {
        run_task_server_or_exit();
        return;
    }

    Application::new().run(|cx: &mut App| {
        cx.open_window(WindowOptions::default(), |_, cx| cx.new(LaReviewApp::new))
            .unwrap();
    });
}

fn run_task_server_or_exit() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime for MCP server");

    let result = runtime.block_on(acp::run_task_mcp_server());
    if let Err(err) = result {
        eprintln!("task MCP server failed: {err:?}");
        process::exit(1);
    }
}
