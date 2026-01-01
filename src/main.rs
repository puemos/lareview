#![allow(unexpected_cfgs)]
//! Main entry point for the LaReview application
//! Initializes the egui application framework and sets up the Tokio runtime.

use eframe::egui;
use lareview::{self, assets, infra, ui};

/// Main entry point for the LaReview application
/// Sets up the Tokio runtime and initializes the egui UI framework
fn main() -> Result<(), eframe::Error> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--printenv") {
        infra::shell::print_env_for_capture();
        return Ok(());
    }

    infra::shell::init_process_path();

    // Initialize logging early - respects RUST_LOG environment variable
    // Usage: RUST_LOG=debug cargo run  (or RUST_LOG=acp=debug for ACP only)
    let _ = env_logger::try_init();

    // Check if we're running as an MCP server
    // Also check for MCP-related environment variables that may indicate MCP mode
    let is_mcp_server = args.contains(&"--task-mcp-server".to_string())
        || std::env::var("MCP_TRANSPORT").is_ok()
        || std::env::var("MCP_SERVER_NAME").is_ok();

    if is_mcp_server {
        // Initialize the global Tokio runtime for MCP mode
        let rt = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("Failed to create Tokio runtime: {e:?}");
                std::process::exit(1);
            }
        };

        if let Err(e) = lareview::set_runtime(rt) {
            eprintln!("Runtime already initialized: {e:?}");
            std::process::exit(1);
        }

        // Enter the runtime context and run the MCP server
        let _guard = lareview::enter_runtime();

        // Run the MCP server instead of the UI
        lareview::block_on(async {
            if let Err(e) = infra::acp::run_task_mcp_server().await {
                eprintln!("MCP server error: {e}");
                std::process::exit(1);
            }
        });

        return Ok(());
    }

    // Initialize the global Tokio runtime for UI mode
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("Failed to create Tokio runtime: {e:?}");
            std::process::exit(1);
        }
    };

    if let Err(e) = lareview::set_runtime(rt) {
        eprintln!("Runtime already initialized: {e:?}");
        std::process::exit(1);
    }

    // Enter the runtime context
    let _guard = lareview::enter_runtime();

    // Load the app icon
    let icon = crate::assets::get_content("assets/logo/512-mac.png")
        .and_then(|bytes| eframe::icon_data::from_png_bytes(bytes).ok());

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1200.0, 800.0])
        .with_titlebar_shown(false)
        .with_title("LaReview")
        .with_fullsize_content_view(true)
        .with_title_shown(false);

    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,

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
