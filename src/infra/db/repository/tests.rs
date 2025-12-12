use super::*;
use crate::domain::TaskStats;
use crate::infra::db::Database;

#[test]
fn test_task_save_and_load() -> anyhow::Result<()> {
    let db = Database::open_at(std::path::PathBuf::from(":memory:"))?;
    let conn = db.connection();
    let repo = TaskRepository::new(conn.clone());
    let pr_repo = PullRequestRepository::new(conn.clone());

    let pr = crate::domain::PullRequest {
        id: "pr-1".to_string(),
        title: "Test PR".to_string(),
        description: None,
        repo: "test/repo".to_string(),
        author: "me".to_string(),
        branch: "main".to_string(),
        created_at: "now".to_string(),
    };
    pr_repo.save(&pr)?;

    let mut task = crate::domain::ReviewTask {
        id: "task-1".to_string(),
        pr_id: pr.id.clone(),
        title: "Test Task".to_string(),
        description: "Desc".to_string(),
        files: vec![],
        stats: TaskStats::default(),
        diffs: vec![],
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
