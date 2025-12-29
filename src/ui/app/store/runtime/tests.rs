use crate::domain::ReviewStatus;
use crate::ui::app::LaReviewApp;
use crate::ui::app::store::runtime::{review, settings};
use std::path::PathBuf;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_update_task_status_runtime() {
    let mut app = LaReviewApp::new_for_test();
    app.skip_runtime = false;

    let task_id = "task-1".to_string();
    review::update_task_status(&mut app, task_id.clone(), ReviewStatus::Done);

    // Polling action messages to process the AsyncAction::TaskStatusSaved
    app.poll_action_messages();

    // We expect an error because the task doesn't exist in the in-memory DB,
    // but the code should have been executed.
}

#[tokio::test]
async fn test_save_and_delete_repo_runtime() {
    let mut app = LaReviewApp::new_for_test();
    app.skip_runtime = false;

    let repo = crate::domain::LinkedRepo {
        id: "repo-1".to_string(),
        name: "Test Repo".to_string(),
        path: PathBuf::from("/tmp/test-repo"),
        remotes: vec![],
        created_at: "2024-01-01T00:00:00Z".to_string(),
    };

    settings::save_repo(&mut app, repo.clone());

    // Wait a bit for tokio::spawn to finish
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    app.poll_action_messages();

    assert!(
        app.state
            .domain
            .linked_repos
            .iter()
            .any(|r| r.id == "repo-1")
    );

    settings::delete_repo(&mut app, "repo-1".to_string());
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    app.poll_action_messages();

    assert!(
        !app.state
            .domain
            .linked_repos
            .iter()
            .any(|r| r.id == "repo-1")
    );
}

#[tokio::test]
async fn test_update_feedback_status_runtime() {
    let mut app = LaReviewApp::new_for_test();
    app.skip_runtime = false;

    // Create review and feedback
    let review = crate::domain::Review {
        id: "rev1".into(),
        title: "T".into(),
        summary: None,
        source: crate::domain::ReviewSource::DiffPaste {
            diff_hash: "h".into(),
        },
        active_run_id: None,
        created_at: "now".into(),
        updated_at: "now".into(),
    };
    app.review_repo.save(&review).unwrap();
    app.state.ui.selected_review_id = Some("rev1".into());

    let feedback = crate::domain::Feedback {
        id: "feedback1".into(),
        review_id: "rev1".into(),
        task_id: None,
        title: "T".into(),
        status: ReviewStatus::Todo,
        impact: crate::domain::FeedbackImpact::Nitpick,
        anchor: None,
        author: "A".into(),
        created_at: "now".into(),
        updated_at: "now".into(),
    };
    app.feedback_repo.save(&feedback).unwrap();

    review::update_feedback_status(&mut app, "feedback1".to_string(), ReviewStatus::Done);
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    app.poll_action_messages();

    let feedback = app.feedback_repo.find_by_id("feedback1").unwrap().unwrap();
    assert_eq!(feedback.status, ReviewStatus::Done);
}

#[tokio::test]
async fn test_update_feedback_impact_runtime() {
    let mut app = LaReviewApp::new_for_test();
    app.skip_runtime = false;

    // Create review first
    let review = crate::domain::Review {
        id: "rev1".into(),
        title: "T".into(),
        summary: None,
        source: crate::domain::ReviewSource::DiffPaste {
            diff_hash: "h".into(),
        },
        active_run_id: None,
        created_at: "now".into(),
        updated_at: "now".into(),
    };
    app.review_repo.save(&review).unwrap();

    let feedback = crate::domain::Feedback {
        id: "feedback1".into(),
        review_id: "rev1".into(),
        task_id: None,
        title: "T".into(),
        status: ReviewStatus::Todo,
        impact: crate::domain::FeedbackImpact::Nitpick,
        anchor: None,
        author: "A".into(),
        created_at: "now".into(),
        updated_at: "now".into(),
    };
    app.feedback_repo.save(&feedback).unwrap();

    review::update_feedback_impact(
        &mut app,
        "feedback1".to_string(),
        crate::domain::FeedbackImpact::Blocking,
    );
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    app.poll_action_messages();
}

#[tokio::test]
async fn test_update_feedback_title_runtime() {
    let mut app = LaReviewApp::new_for_test();
    app.skip_runtime = false;

    // Create review first
    let review = crate::domain::Review {
        id: "rev1".into(),
        title: "T".into(),
        summary: None,
        source: crate::domain::ReviewSource::DiffPaste {
            diff_hash: "h".into(),
        },
        active_run_id: None,
        created_at: "now".into(),
        updated_at: "now".into(),
    };
    app.review_repo.save(&review).unwrap();

    let feedback = crate::domain::Feedback {
        id: "feedback1".into(),
        review_id: "rev1".into(),
        task_id: None,
        title: "T".into(),
        status: ReviewStatus::Todo,
        impact: crate::domain::FeedbackImpact::Nitpick,
        anchor: None,
        author: "A".into(),
        created_at: "now".into(),
        updated_at: "now".into(),
    };
    app.feedback_repo.save(&feedback).unwrap();

    review::update_feedback_title(&mut app, "feedback1".to_string(), "New Title".to_string());
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    app.poll_action_messages();
}

#[test]
fn test_save_app_config_runtime() {
    let tmp_file = NamedTempFile::new().unwrap();
    let path = tmp_file.path().to_path_buf();
    let prev = std::env::var_os("LAREVIEW_CONFIG_PATH");
    unsafe {
        std::env::set_var("LAREVIEW_CONFIG_PATH", &path);
    }

    settings::save_app_config_full(
        true,
        Vec::new(),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        None,
    );

    let contents = std::fs::read_to_string(&path).unwrap();
    assert!(contents.contains("has_seen_requirements = true"));

    match prev {
        Some(value) => unsafe {
            std::env::set_var("LAREVIEW_CONFIG_PATH", value);
        },
        None => unsafe {
            std::env::remove_var("LAREVIEW_CONFIG_PATH");
        },
    }
}

#[tokio::test]
async fn test_abort_generation_runtime() {
    use tokio_util::sync::CancellationToken;
    let mut app = LaReviewApp::new_for_test();
    let token = CancellationToken::new();
    app.agent_cancel_token = Some(token.clone());

    crate::ui::app::store::runtime::generate::abort_generation(&mut app);

    assert!(token.is_cancelled());
    assert!(app.agent_cancel_token.is_none());
}

#[tokio::test]
async fn test_refresh_review_data_runtime() {
    let mut app = LaReviewApp::new_for_test();
    app.skip_runtime = false;

    review::refresh_review_data(
        &mut app,
        crate::ui::app::store::command::ReviewDataRefreshReason::Manual,
    );

    // Wait for async
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    // Poll messages - RefreshReviewData is synchronous but it dispatches an Async action
    app.poll_action_messages();
}

#[tokio::test]
async fn test_create_feedback_comment_runtime() {
    let mut app = LaReviewApp::new_for_test();
    app.skip_runtime = false;

    // 1. Create a review first
    let review = crate::domain::Review {
        id: "rev1".into(),
        title: "T".into(),
        summary: None,
        source: crate::domain::ReviewSource::DiffPaste {
            diff_hash: "h".into(),
        },
        active_run_id: None,
        created_at: "now".into(),
        updated_at: "now".into(),
    };
    app.review_repo.save(&review).unwrap();

    // 1.5 Create a run and a task
    let run = crate::domain::ReviewRun {
        id: "run1".into(),
        review_id: "rev1".into(),
        agent_id: "a".into(),
        input_ref: "ref".into(),
        diff_text: "diff".into(),
        diff_hash: "h".into(),
        created_at: "now".into(),
    };
    app.run_repo.save(&run).unwrap();

    let task = crate::domain::ReviewTask {
        id: "task1".into(),
        run_id: "run1".into(),
        title: "T".into(),
        description: "D".into(),
        files: vec![],
        stats: Default::default(),
        diff_refs: vec![],
        insight: None,
        diagram: None,
        ai_generated: false,
        status: crate::domain::ReviewStatus::Todo,
        sub_flow: None,
    };
    app.task_repo.save(&task).unwrap();

    // 2. Create a comment (new feedback)
    review::create_feedback_comment(
        &mut app,
        "rev1".into(),
        "task1".into(),
        None,
        Some("file.rs".into()),
        Some(1),
        Some("Title".into()),
        "Body".into(),
    );

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    app.poll_action_messages();

    let feedbacks = app.feedback_repo.find_by_review("rev1").unwrap();
    assert_eq!(feedbacks.len(), 1);
    let feedback_id = feedbacks[0].id.clone();

    // 3. Add a reply to existing feedback
    review::create_feedback_comment(
        &mut app,
        "rev1".into(),
        "task1".into(),
        Some(feedback_id),
        None,
        None,
        None,
        "Reply".into(),
    );

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    app.poll_action_messages();
}

#[tokio::test]
async fn test_generate_export_preview_runtime() {
    let mut app = LaReviewApp::new_for_test();
    app.skip_runtime = false;

    // Setup data
    let review = crate::domain::Review {
        id: "r1".into(),
        title: "T".into(),
        summary: None,
        source: crate::domain::ReviewSource::DiffPaste {
            diff_hash: "h".into(),
        },
        active_run_id: Some("run1".into()),
        created_at: "now".into(),
        updated_at: "now".into(),
    };
    app.review_repo.save(&review).unwrap();

    let run = crate::domain::ReviewRun {
        id: "run1".into(),
        review_id: "r1".into(),
        agent_id: "a".into(),
        input_ref: "ref".into(),
        diff_text: "diff".into(),
        diff_hash: "h".into(),
        created_at: "now".into(),
    };
    app.run_repo.save(&run).unwrap();

    review::generate_export_preview(
        &mut app,
        "r1".into(),
        "run1".into(),
        None,
        Default::default(),
    );

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    app.poll_action_messages();
}

#[tokio::test]
async fn test_export_review_runtime() {
    let mut app = LaReviewApp::new_for_test();
    app.skip_runtime = false;

    // Setup data
    let review = crate::domain::Review {
        id: "r1".into(),
        title: "T".into(),
        summary: None,
        source: crate::domain::ReviewSource::DiffPaste {
            diff_hash: "h".into(),
        },
        active_run_id: Some("run1".into()),
        created_at: "now".into(),
        updated_at: "now".into(),
    };
    app.review_repo.save(&review).unwrap();

    let run = crate::domain::ReviewRun {
        id: "run1".into(),
        review_id: "r1".into(),
        agent_id: "a".into(),
        input_ref: "ref".into(),
        diff_text: "diff".into(),
        diff_hash: "h".into(),
        created_at: "now".into(),
    };
    app.run_repo.save(&run).unwrap();

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();

    review::export_review(
        &mut app,
        "r1".into(),
        "run1".into(),
        path,
        Default::default(),
    );

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    app.poll_action_messages();
}

#[tokio::test]
async fn test_refresh_github_review_runtime() {
    let mut app = LaReviewApp::new_for_test();
    app.skip_runtime = false;

    // Error case: review not found
    crate::ui::app::store::runtime::generate::refresh_github_review(
        &mut app,
        "missing".into(),
        "agent1".into(),
    );

    // Error case: review found but not GitHub
    let review = crate::domain::Review {
        id: "rev1".into(),
        title: "T".into(),
        summary: None,
        source: crate::domain::ReviewSource::DiffPaste {
            diff_hash: "h".into(),
        },
        active_run_id: None,
        created_at: "now".into(),
        updated_at: "now".into(),
    };
    app.review_repo.save(&review).unwrap();
    app.state.domain.reviews.push(review);

    crate::ui::app::store::runtime::generate::refresh_github_review(
        &mut app,
        "rev1".into(),
        "agent1".into(),
    );
}

#[tokio::test]
async fn test_resolve_generate_input_runtime() {
    let mut app = LaReviewApp::new_for_test();
    app.skip_runtime = false;

    crate::ui::app::store::runtime::generate::resolve_generate_input(
        &mut app,
        "diff".into(),
        "agent1".into(),
        None,
    );
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    app.poll_action_messages();
}

#[tokio::test]
async fn test_delete_review_runtime() {
    let mut app = LaReviewApp::new_for_test();
    app.skip_runtime = false;

    let review_id = "rev-1".to_string();
    review::delete_review(&mut app, review_id);

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    app.poll_action_messages();
}
