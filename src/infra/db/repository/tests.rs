use crate::domain::{
    Comment, LinkedRepo, Review, ReviewRun, ReviewSource, ReviewStatus, TaskStats, Thread,
    ThreadImpact,
};
use crate::infra::db::Database;
use crate::infra::db::repository::*;
use std::path::PathBuf;

#[test]
fn test_task_repository() -> anyhow::Result<()> {
    let db = Database::open_in_memory()?;
    let conn = db.connection();
    let repo = TaskRepository::new(conn.clone());
    let review_repo = ReviewRepository::new(conn.clone());
    let run_repo = ReviewRunRepository::new(conn.clone());

    let review = Review {
        id: "rev-1".to_string(),
        title: "Test Review".to_string(),
        summary: None,
        source: ReviewSource::DiffPaste {
            diff_hash: "h".into(),
        },
        active_run_id: Some("run-1".into()),
        created_at: "now".to_string(),
        updated_at: "now".to_string(),
    };
    review_repo.save(&review)?;

    let run = ReviewRun {
        id: "run-1".into(),
        review_id: review.id.clone(),
        agent_id: "agent".into(),
        input_ref: "diff".into(),
        diff_text: "diff --git a b".into(),
        diff_hash: "h".into(),
        created_at: "now".into(),
    };
    run_repo.save(&run)?;

    let mut task = crate::domain::ReviewTask {
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
        status: crate::domain::ReviewStatus::Todo,
        sub_flow: None,
    };

    repo.save(&task)?;
    let all = repo.find_all()?;
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, "task-1");

    task.status = ReviewStatus::Done;
    repo.save(&task)?;
    assert_eq!(repo.find_all()?[0].status, ReviewStatus::Done);

    Ok(())
}

#[test]
fn test_repo_repository() -> anyhow::Result<()> {
    let db = Database::open_in_memory()?;
    let repo = RepoRepository::new(db.connection());

    let linked = LinkedRepo {
        id: "repo-1".into(),
        name: "test-repo".into(),
        path: PathBuf::from("/tmp/test"),
        remotes: vec!["https://github.com/test/repo".into()],
        created_at: "now".into(),
    };

    repo.save(&linked)?;
    let all = repo.find_all()?;
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].name, "test-repo");
    assert_eq!(all[0].remotes.len(), 1);

    let found = repo.find_by_remote_url("test/repo")?.expect("found");
    assert_eq!(found.id, "repo-1");

    repo.delete("repo-1")?;
    assert_eq!(repo.find_all()?.len(), 0);

    Ok(())
}

#[test]
fn test_comment_repository() -> anyhow::Result<()> {
    let db = Database::open_in_memory()?;
    let repo = CommentRepository::new(db.connection());
    let thread_repo = ThreadRepository::new(db.connection());
    let review_repo = ReviewRepository::new(db.connection());

    let review = Review {
        id: "rev-1".to_string(),
        title: "Test Review".to_string(),
        summary: None,
        source: ReviewSource::DiffPaste {
            diff_hash: "h".into(),
        },
        active_run_id: None,
        created_at: "now".to_string(),
        updated_at: "now".to_string(),
    };
    review_repo.save(&review)?;

    let thread = Thread {
        id: "t-1".into(),
        review_id: "rev-1".into(),
        task_id: None,
        title: "Thread".into(),
        status: ReviewStatus::Todo,
        impact: ThreadImpact::Nitpick,
        anchor: None,
        author: "me".into(),
        created_at: "now".into(),
        updated_at: "now".into(),
    };
    thread_repo.save(&thread)?;

    let comment = Comment {
        id: "c-1".into(),
        thread_id: "t-1".into(),
        author: "me".into(),
        body: "hello".into(),
        parent_id: None,
        created_at: "now".into(),
        updated_at: "now".into(),
    };

    repo.save(&comment)?;
    let list = repo.list_for_thread("t-1")?;
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].body, "hello");

    repo.touch("c-1")?;
    repo.delete_by_thread("t-1")?;
    assert_eq!(repo.list_for_thread("t-1")?.len(), 0);

    Ok(())
}

#[test]
fn test_thread_repository() -> anyhow::Result<()> {
    let db = Database::open_in_memory()?;
    let repo = ThreadRepository::new(db.connection());
    let review_repo = ReviewRepository::new(db.connection());

    let review = Review {
        id: "rev-1".to_string(),
        title: "Test Review".to_string(),
        summary: None,
        source: ReviewSource::DiffPaste {
            diff_hash: "h".into(),
        },
        active_run_id: None,
        created_at: "now".to_string(),
        updated_at: "now".to_string(),
    };
    review_repo.save(&review)?;

    let thread = Thread {
        id: "t-1".into(),
        review_id: "rev-1".into(),
        task_id: None,
        title: "Thread".into(),
        status: ReviewStatus::Todo,
        impact: ThreadImpact::Nitpick,
        anchor: None,
        author: "me".into(),
        created_at: "now".into(),
        updated_at: "now".into(),
    };

    repo.save(&thread)?;
    let list = repo.find_by_review("rev-1")?;
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].title, "Thread");

    repo.update_status("t-1", ReviewStatus::Done)?;
    repo.update_impact("t-1", ThreadImpact::Blocking)?;
    repo.update_title("t-1", "New Title")?;
    repo.touch("t-1")?;

    let updated = repo.find_by_review("rev-1")?;
    assert_eq!(updated[0].status, ReviewStatus::Done);
    assert_eq!(updated[0].impact, ThreadImpact::Blocking);
    assert_eq!(updated[0].title, "New Title");

    repo.delete_by_review("rev-1")?;
    assert_eq!(repo.find_by_review("rev-1")?.len(), 0);

    Ok(())
}

#[test]
fn test_review_run_repository() -> anyhow::Result<()> {
    let db = Database::open_in_memory()?;
    let repo = ReviewRunRepository::new(db.connection());
    let review_repo = ReviewRepository::new(db.connection());

    let review = Review {
        id: "rev-1".to_string(),
        title: "Test Review".to_string(),
        summary: None,
        source: ReviewSource::DiffPaste {
            diff_hash: "h".into(),
        },
        active_run_id: None,
        created_at: "now".to_string(),
        updated_at: "now".to_string(),
    };
    review_repo.save(&review)?;

    let run = ReviewRun {
        id: "run-1".into(),
        review_id: "rev-1".into(),
        agent_id: "agent".into(),
        input_ref: "input".into(),
        diff_text: "diff".into(),
        diff_hash: "h".into(),
        created_at: "now".into(),
    };

    repo.save(&run)?;
    assert!(repo.find_by_id(&"run-1".into())?.is_some());
    assert_eq!(repo.find_by_review_id(&"rev-1".into())?.len(), 1);
    assert_eq!(repo.list_all()?.len(), 1);

    repo.delete_by_review_id(&"rev-1".into())?;
    assert_eq!(repo.list_all()?.len(), 0);

    Ok(())
}

#[test]
fn test_review_repository() -> anyhow::Result<()> {
    let db = Database::open_in_memory()?;
    let repo = ReviewRepository::new(db.connection());

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

    repo.save(&review)?;
    assert!(repo.find_by_id(&"rev-1".into())?.is_some());
    assert_eq!(repo.list_all()?.len(), 1);

    repo.update_title_and_summary(&"rev-1".into(), "New Title", Some("New Summary"))?;
    repo.set_active_run(&"rev-1".into(), &"run-1".into())?;

    let updated = repo.find_by_id(&"rev-1".into())?.unwrap();
    assert_eq!(updated.title, "New Title");
    assert_eq!(updated.summary, Some("New Summary".into()));
    assert_eq!(updated.active_run_id, Some("run-1".into()));

    repo.delete(&"rev-1".into())?;
    assert_eq!(repo.list_all()?.len(), 0);

    Ok(())
}
