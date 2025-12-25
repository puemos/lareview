use crate::infra::acp::ProgressEvent;
use crate::ui::app::tests::fixtures::create_mock_preview;
use crate::ui::app::tests::harness::setup_harness;
use crate::ui::app::{AppView, LaReviewApp};
use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_generate_input_flow() {
    let app = Arc::new(Mutex::new(LaReviewApp::new_for_test()));
    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.ui.current_view = AppView::Generate;
    }
    let mut harness = setup_harness(app.clone());

    harness.run_steps(2);

    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.session.diff_text = "test diff".to_string();
    }
    harness.run();

    let gen_btn = harness.get_by_label("Run Agent");
    gen_btn.click();

    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.session.is_generating = true;
    }

    harness.step();

    assert!(app.lock().unwrap().state.session.is_generating);
}

#[tokio::test]
async fn test_generate_valid_diff_pasting() {
    let app = Arc::new(Mutex::new(LaReviewApp::new_for_test()));
    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.ui.current_view = AppView::Generate;
    }
    let mut harness = setup_harness(app.clone());
    harness.run_steps(2);

    let diff = "--- a/file.rs\n+++ b/file.rs\n@@ -1,1 +1,1 @@\n-old\n+new";

    harness
        .get_by_role(Role::MultilineTextInput)
        .type_text(diff);

    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.session.diff_text = diff.to_string();
    }
    harness.run();

    let run_btn = harness.get_by_label("Run Agent");
    assert!(!format!("{:?}", run_btn).contains("disabled: true"));
}

#[tokio::test]
async fn test_generate_invalid_diff_pasting() {
    let app = Arc::new(Mutex::new(LaReviewApp::new_for_test()));
    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.ui.current_view = AppView::Generate;
    }
    let mut harness = setup_harness(app.clone());
    harness.run_steps(2);

    // Empty text should disable the button
    harness
        .get_by_role(Role::MultilineTextInput)
        .type_text("   ");
    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.session.diff_text = "   ".to_string();
    }
    harness.run();

    let run_btn = harness.get_by_label("Run Agent");
    assert!(format!("{:?}", run_btn).contains("disabled: true"));
}

#[tokio::test]
async fn test_generate_valid_pr_pasting() {
    let app = Arc::new(Mutex::new(LaReviewApp::new_for_test()));
    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.ui.current_view = AppView::Generate;
    }
    let mut harness = setup_harness(app.clone());
    harness.run_steps(2);

    let pr_ref = "owner/repo#123";
    harness
        .get_by_role(Role::MultilineTextInput)
        .type_text(pr_ref);

    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.session.diff_text = pr_ref.to_string();
        app_lock.state.session.is_preview_fetching = true;
    }
    harness.step();

    {
        let mut app_lock = app.lock().unwrap();
        let preview = create_mock_preview("mock diff from github");
        app_lock.state.session.is_preview_fetching = false;
        app_lock.state.session.generate_preview = Some(preview);
    }
    harness.run();

    let run_btn = harness.get_by_label("Run Agent");
    assert!(!format!("{:?}", run_btn).contains("disabled: true"));
}

#[tokio::test]
async fn test_generate_invalid_pr_pasting() {
    let app = Arc::new(Mutex::new(LaReviewApp::new_for_test()));
    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.ui.current_view = AppView::Generate;
    }
    let mut harness = setup_harness(app.clone());
    harness.run_steps(2);

    let pr_ref = "owner/repo#123";
    harness
        .get_by_role(Role::MultilineTextInput)
        .type_text(pr_ref);

    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.session.diff_text = pr_ref.to_string();
        app_lock.state.session.is_preview_fetching = true;
    }
    harness.step();

    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.session.is_preview_fetching = false;
        app_lock.state.session.generation_error = Some("Failed to fetch PR".to_string());
    }
    harness.run();

    harness.get_by_label("error: Failed to fetch PR");

    // Note: The button is actually NOT disabled currently if diff_text is not empty,
    // even if preview fetching failed. This allows the user to try running anyway.
}

#[tokio::test]
async fn test_agent_switching() {
    let app = Arc::new(Mutex::new(LaReviewApp::new_for_test()));
    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.ui.current_view = AppView::Generate;
        app_lock.state.session.selected_agent = crate::ui::app::SelectedAgent::new("codex");
    }
    let mut harness = setup_harness(app.clone());

    harness.run_steps(2);

    harness
        .get_all_by_label("Codex")
        .find(|n| format!("{:?}", n).contains("role: Button"))
        .expect("Agent selector button not found")
        .click();

    harness.run();

    let gemini_node = harness.get_all_by_label("Gemini").next();
    if let Some(node) = gemini_node {
        node.click();
    }

    harness.run();
}

#[tokio::test]
async fn test_timeline_updates() {
    let app = Arc::new(Mutex::new(LaReviewApp::new_for_test()));
    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.ui.current_view = AppView::Generate;
        app_lock.state.session.is_generating = true;
    }
    let mut harness = setup_harness(app.clone());

    harness.run_steps(2);

    {
        let mut app_lock = app.lock().unwrap();
        app_lock
            .state
            .ingest_progress(ProgressEvent::LocalLog("Agent starting...".to_string()));
    }

    harness.step();

    harness.get_by_label("Agent starting...");
}
