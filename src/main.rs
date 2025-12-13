//! Main entry point for the LaReview application
//! Initializes the egui application framework and sets up the Tokio runtime.

mod application;
mod domain;
mod infra;
mod prompts;
mod ui;

use eframe::egui;
use std::sync::OnceLock;

/// Global Tokio runtime handle for async operations throughout the application
static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

/// Main entry point for the LaReview application
/// Sets up the Tokio runtime and initializes the egui UI framework
fn main() -> Result<(), eframe::Error> {
    let args: Vec<String> = std::env::args().collect();

    // Check if we're running as an MCP server
    // Also check for MCP-related environment variables that may indicate MCP mode
    let is_mcp_server = args.contains(&"--task-mcp-server".to_string())
        || std::env::var("MCP_TRANSPORT").is_ok()
        || std::env::var("MCP_SERVER_NAME").is_ok();

    if is_mcp_server {
        // Initialize the global Tokio runtime
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");

        RUNTIME.set(rt).expect("Runtime already initialized");

        // Enter the runtime context and run the MCP server
        let _guard = RUNTIME.get().unwrap().enter();

        // Run the MCP server instead of the UI
        let rt = RUNTIME.get().unwrap();
        rt.block_on(async {
            if let Err(e) = infra::acp::run_task_mcp_server().await {
                eprintln!("MCP server error: {e}");
                std::process::exit(1);
            }
        });

        return Ok(());
    }

    // Initialize the global Tokio runtime for UI mode
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    RUNTIME.set(rt).expect("Runtime already initialized");

    // Enter the runtime context
    let _guard = RUNTIME.get().unwrap().enter();

    // Load the app icon
    let icon = eframe::icon_data::from_png_bytes(
        &std::fs::read("assets/icons/icon-512.png").expect("Failed to read app icon file"),
    )
    .expect("Failed to decode app icon");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("LaReview")
            .with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        "LaReview",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(ui::app::LaReviewApp::new_egui(cc)))
        }),
    )
}
