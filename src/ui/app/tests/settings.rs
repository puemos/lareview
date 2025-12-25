use crate::ui::app::LaReviewApp;
use crate::ui::app::state::AppView;
use crate::ui::app::tests::harness::setup_harness;
use egui_kittest::kittest::Queryable;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_settings_extra_path_persistence() {
    let app = Arc::new(Mutex::new(LaReviewApp::new_for_test()));
    let mut harness = setup_harness(app.clone());

    harness.run_steps(2);
    app.lock().unwrap().state.ui.current_view = AppView::Settings;
    harness.run();

    let new_path = "/custom/path".to_string();
    app.lock().unwrap().state.ui.extra_path = new_path.clone();

    harness.run();
    assert_eq!(app.lock().unwrap().state.ui.extra_path, new_path);
}

#[tokio::test]
async fn test_requirements_modal_interaction() {
    let app = Arc::new(Mutex::new(LaReviewApp::new_for_test()));
    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.ui.show_requirements_modal = true;
    }
    let mut harness = setup_harness(app.clone());

    harness.run_steps(2);

    // Verify modal is visible
    harness.get_by_label("Setup Checklist");

    // Click "Dismiss"
    harness.get_by_label("Dismiss").click();
    harness.run();

    assert!(!app.lock().unwrap().state.ui.show_requirements_modal);
    assert!(app.lock().unwrap().state.ui.has_seen_requirements);
}

#[tokio::test]
async fn test_settings_d2_toggle() {
    let app = Arc::new(Mutex::new(LaReviewApp::new_for_test()));
    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.ui.current_view = AppView::Settings;
        app_lock.state.ui.allow_d2_install = false;
    }
    let mut harness = setup_harness(app.clone());

    harness.run_steps(2);

    let d2_installed = crate::infra::brew::find_bin("d2").is_some();
    if !d2_installed {
        harness
            .get_by_label("I understand and want to proceed")
            .click();
        harness.run();
        assert!(app.lock().unwrap().state.ui.allow_d2_install);
    } else {
        harness.get_by_label("✔ Installed");
    }
}
#[tokio::test]
async fn test_settings_gh_refresh() {
    let app = Arc::new(Mutex::new(LaReviewApp::new_for_test()));
    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.ui.current_view = AppView::Settings;
    }
    let mut harness = setup_harness(app.clone());

    harness.run_steps(2);

    harness.get_by_label("Refresh Status").click();
    harness.run();

    assert!(app.lock().unwrap().state.session.is_gh_status_checking);
}
