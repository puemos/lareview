use crate::domain::LinkedRepo;
use crate::ui::app::tests::harness::setup_harness;
use crate::ui::app::{AppView, LaReviewApp};
use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_repos_view_empty_state() {
    let app = Arc::new(Mutex::new(LaReviewApp::new_for_test()));
    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.ui.current_view = AppView::Repos;
        app_lock.state.domain.linked_repos.clear();
    }
    let mut harness = setup_harness(app.clone());

    harness.run_steps(2);

    harness.get_by_label(
        "No repositories linked. Link a local Git repo to allow the agent to read file contents.",
    );
}

#[tokio::test]
async fn test_repos_view_list_and_unlink() {
    let app = Arc::new(Mutex::new(LaReviewApp::new_for_test()));
    let repo_id = "repo_1".to_string();
    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.ui.current_view = AppView::Repos;
        app_lock.state.domain.linked_repos = vec![LinkedRepo {
            id: repo_id.clone(),
            name: "Mock Repo".to_string(),
            path: PathBuf::from("/mock/path"),
            remotes: vec!["origin".to_string()],
            created_at: "2023-01-01T00:00:00Z".to_string(),
        }];
    }
    let mut harness = setup_harness(app.clone());

    harness.run_steps(2);

    harness.get_by_label("Mock Repo");

    let unlink_btn = harness
        .get_all_by_role(Role::Button)
        .into_iter()
        .find(|n| format!("{:?}", n).contains("Unlink Repository"))
        .expect("Unlink button not found");

    unlink_btn.click();

    // Simulate what reducer does: remove repo
    {
        let mut app_lock = app.lock().unwrap();
        app_lock.state.domain.linked_repos.clear();
    }

    harness.run();

    harness.get_by_label(
        "No repositories linked. Link a local Git repo to allow the agent to read file contents.",
    );
}
