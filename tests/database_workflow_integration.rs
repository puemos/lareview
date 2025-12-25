//! Integration tests for the database functionality
//! These tests verify that different repository modules work together correctly

use lareview::domain::{
    Comment, LinkedRepo, Review, ReviewRun, ReviewSource, ReviewStatus, TaskStats, Thread,
    ThreadImpact,
};
use lareview::infra::db::{Database, repository::*};
use std::path::PathBuf;

#[test]
fn test_full_database_workflow() -> anyhow::Result<()> {
    // Test that all repository modules work together in a full workflow
    let db = Database::open_in_memory()?;
    let conn = db.connection();

    // Get all repositories
    let review_repo = ReviewRepository::new(conn.clone());
    let run_repo = ReviewRunRepository::new(conn.clone());
    let task_repo = TaskRepository::new(conn.clone());
    let thread_repo = ThreadRepository::new(conn.clone());
    let comment_repo = CommentRepository::new(conn.clone());
    let repo_repo = RepoRepository::new(conn.clone());

    // Create a linked repo
    let linked = LinkedRepo {
        id: "repo-1".into(),
        name: "test-repo".into(),
        path: PathBuf::from("/tmp/test"),
        remotes: vec!["https://github.com/test/repo".into()],
        created_at: "now".into(),
    };
    repo_repo.save(&linked)?;

    // Create a review
    let review = Review {
        id: "rev-1".to_string(),
        title: "Test Review".to_string(),
        summary: Some("Summary".into()),
        source: ReviewSource::DiffPaste {
            diff_hash: "h".into(),
        },
        active_run_id: None,
        created_at: "now".to_string(),
        updated_at: "now".to_string(),
    };
    review_repo.save(&review)?;

    // Create a review run
    let run = ReviewRun {
        id: "run-1".into(),
        review_id: "rev-1".into(),
        agent_id: "agent".into(),
        input_ref: "input".into(),
        diff_text: "diff".into(),
        diff_hash: "h".into(),
        created_at: "now".into(),
    };
    run_repo.save(&run)?;

    // Update review to use this run
    review_repo.set_active_run(&review.id, &run.id)?;

    // Create a task
    let task = lareview::domain::ReviewTask {
        id: "task-1".to_string(),
        run_id: run.id.clone(),
        title: "Test Task".to_string(),
        description: "Desc".to_string(),
        files: vec![],
        stats: TaskStats::default(),
        diff_refs: vec![],
        insight: None,
        diagram: None,
        ai_generated: false,
        status: ReviewStatus::Todo,
        sub_flow: None,
    };
    task_repo.save(&task)?;

    // Create a thread
    let thread = Thread {
        id: "t-1".into(),
        review_id: review.id.clone(),
        task_id: Some(task.id.clone()),
        title: "Thread".into(),
        status: ReviewStatus::Todo,
        impact: ThreadImpact::Nitpick,
        anchor: None,
        author: "me".into(),
        created_at: "now".into(),
        updated_at: "now".into(),
    };
    thread_repo.save(&thread)?;

    // Create a comment
    let comment = Comment {
        id: "c-1".into(),
        thread_id: thread.id.clone(),
        author: "me".into(),
        body: "hello".into(),
        parent_id: None,
        created_at: "now".into(),
        updated_at: "now".into(),
    };
    comment_repo.save(&comment)?;

    // Verify all data can be retrieved correctly
    let retrieved_review = review_repo.find_by_id(&review.id)?.unwrap();
    assert_eq!(retrieved_review.title, "Test Review");

    let retrieved_run = run_repo.find_by_id(&run.id)?.unwrap();
    assert_eq!(retrieved_run.agent_id, "agent");

    let all_tasks = task_repo.find_all()?;
    assert_eq!(all_tasks.len(), 1);

    let all_threads = thread_repo.find_by_review(&review.id)?;
    assert_eq!(all_threads.len(), 1);

    let thread_comments = comment_repo.list_for_thread(&thread.id)?;
    assert_eq!(thread_comments.len(), 1);
    assert_eq!(thread_comments[0].body, "hello");

    // Test that repositories can be used together
    let review_tasks = task_repo.find_by_run(&run.id)?;
    assert_eq!(review_tasks.len(), 1);

    let review_threads = thread_repo.find_by_review(&review.id)?;
    assert_eq!(review_threads.len(), 1);

    Ok(())
}
