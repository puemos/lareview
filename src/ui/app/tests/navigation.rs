use crate::ui::app::LaReviewApp;
use crate::ui::app::state::AppView;
use crate::ui::app::tests::harness::setup_harness;
use egui_kittest::kittest::Queryable;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_navigation_flow() {
    let app = Arc::new(Mutex::new(LaReviewApp::new_for_test()));
    let mut harness = setup_harness(app.clone());

    harness.run_steps(2);
    assert_eq!(app.lock().unwrap().state.ui.current_view, AppView::Home);

    app.lock().unwrap().state.ui.current_view = AppView::Generate;
    harness.run();
    assert_eq!(app.lock().unwrap().state.ui.current_view, AppView::Generate);

    app.lock().unwrap().state.ui.current_view = AppView::Review;
    harness.run();
    assert_eq!(app.lock().unwrap().state.ui.current_view, AppView::Review);
}

#[tokio::test]
async fn test_click_navigation() {
    let app = Arc::new(Mutex::new(LaReviewApp::new_for_test()));
    let mut harness = setup_harness(app.clone());

    harness.run_steps(2);
    assert_eq!(app.lock().unwrap().state.ui.current_view, AppView::Home);

    harness.get_by_label("Settings").click();
    harness.run();
    assert_eq!(app.lock().unwrap().state.ui.current_view, AppView::Settings);

    harness.get_by_label("Generate").click();
    harness.run();
    assert_eq!(app.lock().unwrap().state.ui.current_view, AppView::Generate);
}

#[tokio::test]
async fn test_header_identity_rendered() {
    let app = Arc::new(Mutex::new(LaReviewApp::new_for_test()));
    let mut harness = setup_harness(app.clone());

    harness.run_steps(2);
    harness.get_by_label("LaReview");
}
