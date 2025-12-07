mod acp;
mod data;
mod domain;
mod ui;

use eframe::egui;
use std::sync::OnceLock;

// Global Tokio runtime handle
static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn main() -> Result<(), eframe::Error> {
    // Initialize the global Tokio runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    RUNTIME.set(rt).expect("Runtime already initialized");

    // Enter the runtime context
    let _guard = RUNTIME.get().unwrap().enter();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("LaReview"),
        ..Default::default()
    };

    eframe::run_native(
        "LaReview",
        options,
        Box::new(|cc| Ok(Box::new(ui::app::LaReviewApp::new_egui(cc)))),
    )
}
