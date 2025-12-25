use crate::ui::app::tests::fixtures::populate_app_with_mock_data;
use crate::ui::app::tests::harness::setup_harness;
use crate::ui::app::{AppView, LaReviewApp};
use egui_kittest::kittest::Queryable;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_home_to_review_transition() {
    let app = Arc::new(Mutex::new(LaReviewApp::new_for_test()));
    {
        let mut app_lock = app.lock().unwrap();
        populate_app_with_mock_data(&mut app_lock);
        app_lock.state.ui.current_view = AppView::Home;
    }
    let mut harness = setup_harness(app.clone());

    harness.run_steps(2);

    harness.get_by_label("Sample Review");
    let open_btn = harness.get_by_label("Open");

    open_btn.click();

    // Multiple runs to allow reducer to process
    for _ in 0..5 {
        harness.run();
    }

    assert_eq!(app.lock().unwrap().state.ui.current_view, AppView::Review);
    assert_eq!(
        app.lock().unwrap().state.ui.selected_review_id.as_deref(),
        Some("rev_1")
    );
}
