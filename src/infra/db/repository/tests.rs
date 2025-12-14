use super::*;
use crate::domain::TaskStats;
use crate::domain::{Review, ReviewRun, ReviewSource};
use crate::infra::db::Database;

#[test]
fn test_task_save_and_load() -> anyhow::Result<()> {
    let db = Database::open_at(std::path::PathBuf::from(":memory:"))?;
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
        status: crate::domain::TaskStatus::Pending,
        sub_flow: None,
    };

    repo.save(&task)?;

    let all_tasks = repo.find_all()?;
    assert_eq!(all_tasks.len(), 1);
    assert_eq!(all_tasks[0].status, crate::domain::TaskStatus::Pending);

    task.status = crate::domain::TaskStatus::Done;
    repo.save(&task)?;

    let all_tasks = repo.find_all()?;
    assert_eq!(all_tasks.len(), 1);
    assert_eq!(all_tasks[0].status, crate::domain::TaskStatus::Done);

    Ok(())
}

#[test]
fn test_note_repository_round_trip() -> anyhow::Result<()> {
    let db = Database::open_at(std::path::PathBuf::from(":memory:"))?;
    let conn = db.connection();
    let note_repo = NoteRepository::new(conn.clone());

    let task_id = "task-note-1".to_string();
    let note = crate::domain::Note {
        task_id: task_id.clone(),
        body: "Body".into(),
        updated_at: "now".into(),
        file_path: None,
        line_number: None,
    };

    note_repo.save(&note)?;
    let fetched = note_repo.find_by_task(&task_id)?;
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().body, "Body");

    Ok(())
}
