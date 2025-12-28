use crate::ui::app::tests::harness::setup_harness;
use crate::ui::app::{FullDiffView, LaReviewApp};
use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_full_diff_overlay_flow() {
    let app = Arc::new(Mutex::new(LaReviewApp::new_for_test()));
    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.ui.full_diff = Some(FullDiffView {
            title: "Overlay Diff".to_string(),
            text: Arc::from("diff content"),
        });
    }
    let mut harness = setup_harness(app.clone());

    harness.run_steps(2);

    // Check if window with title exists
    harness.get_by_label("Overlay Diff");

    // Click Close button
    harness
        .get_all_by_role(Role::Button)
        .into_iter()
        .find(|n| format!("{:?}", n).contains("Close"))
        .expect("Close button not found")
        .click();

    // Simulate reducer closing it
    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.ui.full_diff = None;
    }
    harness.run();
}

#[tokio::test]
async fn test_export_preview_overlay_flow() {
    let app = Arc::new(Mutex::new(LaReviewApp::new_for_test()));
    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.ui.export_preview = Some("# Export Preview".to_string());
    }
    let mut harness = setup_harness(app.clone());

    harness.run_steps(2);

    // Use Role::Window to uniquely identify the modal
    harness
        .get_all_by_role(Role::Window)
        .into_iter()
        .find(|n| format!("{:?}", n).contains("Export Review"))
        .expect("Export Preview Window not found");

    // Find Cancel button
    harness
        .get_all_by_role(Role::Button)
        .into_iter()
        .find(|n| format!("{:?}", n).contains("Cancel"))
        .expect("Cancel button not found")
        .click();

    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.ui.export_preview = None;
    }
    harness.run();
}
