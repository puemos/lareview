use crate::domain::*;
use crate::ui::app::LaReviewApp;
use crate::ui::app::state::AppState;
use std::sync::Arc;

pub fn create_mock_review(id: &str, title: &str) -> Review {
    Review {
        id: id.to_string(),
        title: title.to_string(),
        summary: Some("Mock summary".to_string()),
        source: ReviewSource::DiffPaste {
            diff_hash: "mock_hash".to_string(),
        },
        active_run_id: Some(format!("{}_run", id)),
        created_at: "2023-01-01T00:00:00Z".to_string(),
        updated_at: "2023-01-01T00:00:00Z".to_string(),
    }
}

pub fn create_mock_run(review_id: &str, run_id: &str) -> ReviewRun {
    ReviewRun {
        id: run_id.to_string(),
        review_id: review_id.to_string(),
        agent_id: "mock_agent".to_string(),
        input_ref: "mock_input".to_string(),
        diff_text: Arc::from("--- a/file.txt\n+++ b/file.txt\n@@ -1,1 +1,1 @@\n-old\n+new"),
        diff_hash: "mock_hash".to_string(),
        created_at: "2023-01-01T00:00:00Z".to_string(),
    }
}

pub fn create_mock_task(run_id: &str, task_id: &str, title: &str) -> ReviewTask {
    ReviewTask {
        id: task_id.to_string(),
        run_id: run_id.to_string(),
        title: title.to_string(),
        description: "Mock task description".to_string(),
        files: vec!["file.txt".to_string()],
        stats: TaskStats {
            additions: 1,
            deletions: 1,
            risk: RiskLevel::Low,
            tags: vec!["mock".to_string()],
        },
        diff_refs: vec![DiffRef {
            file: "file.txt".to_string(),
            hunks: vec![HunkRef {
                old_start: 1,
                old_lines: 1,
                new_start: 1,
                new_lines: 1,
            }],
        }],
        insight: Some(Arc::from("Mock insight")),
        diagram: None,
        ai_generated: true,
        status: ReviewStatus::Todo,
        sub_flow: None,
    }
}

pub fn populate_app_with_mock_data(app: &mut LaReviewApp) {
    let review_id = "rev_1";
    let run_id = "rev_1_run";

    let review = create_mock_review(review_id, "Sample Review");
    let run = create_mock_run(review_id, run_id);
    let task1 = create_mock_task(run_id, "task_1", "First Task");
    let task2 = create_mock_task(run_id, "task_2", "Second Task");

    // Save to DB
    app.review_repo.save(&review).unwrap();
    app.run_repo.save(&run).unwrap();
    app.task_repo.save(&task1).unwrap();
    app.task_repo.save(&task2).unwrap();

    // Populate state
    app.state.domain.reviews.push(review);
    app.state.domain.runs.push(run);
    app.state.domain.all_tasks.push(task1);
    app.state.domain.all_tasks.push(task2);

    app.state.ui.selected_review_id = Some(review_id.to_string());
    app.state.ui.selected_run_id = Some(run_id.to_string());
    app.state.ui.selected_task_id = Some("task_1".to_string());
    app.state.ui.current_view = crate::ui::app::state::AppView::Review;
}

pub fn populate_state_with_mock_data(state: &mut AppState) {
    let review_id = "rev_1";
    let run_id = "rev_1_run";

    let review = create_mock_review(review_id, "Sample Review");
    let run = create_mock_run(review_id, run_id);
    let task1 = create_mock_task(run_id, "task_1", "First Task");
    let task2 = create_mock_task(run_id, "task_2", "Second Task");

    state.domain.reviews.push(review);
    state.domain.runs.push(run);
    state.domain.all_tasks.push(task1);
    state.domain.all_tasks.push(task2);

    state.ui.selected_review_id = Some(review_id.to_string());
    state.ui.selected_run_id = Some(run_id.to_string());
    state.ui.selected_task_id = Some("task_1".to_string());
    state.ui.current_view = crate::ui::app::state::AppView::Review;
}

pub fn create_mock_preview(diff_text: &str) -> crate::ui::app::state::GeneratePreview {
    crate::ui::app::state::GeneratePreview {
        diff_text: Arc::from(diff_text),
        github: None,
    }
}
