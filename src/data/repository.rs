#![allow(dead_code)]
//! Repository traits and implementations for data access

use crate::domain::{Note, PullRequest, PullRequestId, ReviewTask, TaskId, TaskStatus};
use anyhow::Result;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Repository for pull request operations
pub struct PullRequestRepository {
    conn: Arc<Mutex<Connection>>,
}

impl PullRequestRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn save(&self, pr: &PullRequest) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT OR REPLACE INTO pull_requests (id, title, description, repo, author, branch, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            (
                &pr.id,
                &pr.title,
                &pr.description,
                &pr.repo,
                &pr.author,
                &pr.branch,
                &pr.created_at,
            ),
        )?;
        Ok(())
    }

    pub fn find_by_id(&self, id: &PullRequestId) -> Result<Option<PullRequest>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, description, repo, author, branch, created_at FROM pull_requests WHERE id = ?1",
        )?;

        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(PullRequest {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                repo: row.get(3)?,
                author: row.get(4)?,
                branch: row.get(5)?,
                created_at: row.get(6)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_all(&self) -> Result<Vec<PullRequest>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, description, repo, author, branch, created_at FROM pull_requests ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(PullRequest {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                repo: row.get(3)?,
                author: row.get(4)?,
                branch: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}

/// Repository for task operations
pub struct TaskRepository {
    conn: Arc<Mutex<Connection>>,
}

impl TaskRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn save(&self, pr_id: &PullRequestId, task: &ReviewTask) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let files_json = serde_json::to_string(&task.files)?;
        let stats_json = serde_json::to_string(&task.stats)?;
        let patches_json = serde_json::to_string(&task.patches)?;

        let status_str = serde_json::to_string(&task.status)?.replace("\"", "");

        conn.execute(
            r#"
            INSERT OR REPLACE INTO tasks (id, pull_request_id, title, description, files, stats, insight, patches, diagram, ai_generated, status)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            (
                &task.id,
                pr_id,
                &task.title,
                &task.description,
                &files_json,
                &stats_json,
                &task.insight,
                &patches_json,
                &task.diagram,
                task.ai_generated as i32,
                &status_str,
            ),
        )?;
        Ok(())
    }

    pub fn find_by_pr(&self, pr_id: &PullRequestId) -> Result<Vec<ReviewTask>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, description, files, stats, insight, patches, diagram, ai_generated, status FROM tasks WHERE pull_request_id = ?1",
        )?;

        let rows = stmt.query_map([pr_id], |row| {
            let files_json: String = row.get(3)?;
            let stats_json: String = row.get(4)?;
            let patches_json: Option<String> = row.get(6)?;
            let status_str: String = row.get(9)?;

            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                files_json,
                stats_json,
                row.get::<_, Option<String>>(5)?,
                patches_json,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, i32>(8)?,
                status_str,
            ))
        })?;

        let mut tasks = Vec::new();
        for row in rows {
            let (
                id,
                title,
                description,
                files_json,
                stats_json,
                insight,
                patches_json,
                diagram,
                ai_generated,
                status_str,
            ) = row?;

            let status = match status_str.as_str() {
                "REVIEWED" => TaskStatus::Reviewed,
                "IGNORED" => TaskStatus::Ignored,
                _ => TaskStatus::Pending,
            };

            tasks.push(ReviewTask {
                id,
                title,
                description,
                files: serde_json::from_str(&files_json).unwrap_or_default(),
                stats: serde_json::from_str(&stats_json).unwrap_or_default(),
                insight,
                patches: patches_json
                    .map(|s| serde_json::from_str(&s).unwrap_or_default())
                    .unwrap_or_default(),
                diagram,
                ai_generated: ai_generated != 0,
                status,
            });
        }
        Ok(tasks)
    }

    pub fn update_status(&self, task_id: &TaskId, status: TaskStatus) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let status_str = serde_json::to_string(&status)?.replace("\"", "");

        conn.execute(
            "UPDATE tasks SET status = ?1 WHERE id = ?2",
            (&status_str, task_id),
        )?;
        Ok(())
    }
}

/// Repository for note operations
pub struct NoteRepository {
    conn: Arc<Mutex<Connection>>,
}

impl NoteRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn save(&self, note: &Note) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO notes (task_id, body, updated_at) VALUES (?1, ?2, ?3)",
            (&note.task_id, &note.body, &note.updated_at),
        )?;
        Ok(())
    }

    pub fn find_by_task(&self, task_id: &TaskId) -> Result<Option<Note>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT task_id, body, updated_at FROM notes WHERE task_id = ?1")?;

        let mut rows = stmt.query([task_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Note {
                task_id: row.get(0)?,
                body: row.get(1)?,
                updated_at: row.get(2)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn find_by_tasks(&self, task_ids: &[TaskId]) -> Result<Vec<Note>> {
        if task_ids.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.conn.lock().unwrap();
        let placeholders: Vec<_> = (1..=task_ids.len()).map(|i| format!("?{}", i)).collect();
        let query = format!(
            "SELECT task_id, body, updated_at FROM notes WHERE task_id IN ({})",
            placeholders.join(", ")
        );

        let mut stmt = conn.prepare(&query)?;
        let params: Vec<&dyn rusqlite::ToSql> =
            task_ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

        let rows = stmt.query_map(params.as_slice(), |row| {
            Ok(Note {
                task_id: row.get(0)?,
                body: row.get(1)?,
                updated_at: row.get(2)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn delete_by_task(&self, task_id: &TaskId) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM notes WHERE task_id = ?1", [task_id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::db::Database;
    use crate::domain::TaskStats;

    #[test]
    fn test_task_status_persistence() -> Result<()> {
        let db = Database::open_at(std::path::PathBuf::from(":memory:"))?;
        let conn = db.connection();
        let repo = TaskRepository::new(conn.clone());
        let pr_repo = PullRequestRepository::new(conn.clone());

        let pr = PullRequest {
            id: "pr-1".to_string(),
            title: "Test PR".to_string(),
            description: None,
            repo: "test/repo".to_string(),
            author: "me".to_string(),
            branch: "main".to_string(),
            created_at: "now".to_string(),
        };
        pr_repo.save(&pr)?;

        let task = ReviewTask {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            description: "Desc".to_string(),
            files: vec![],
            stats: TaskStats::default(),
            patches: vec![],
            insight: None,
            diagram: None,
            ai_generated: false,
            status: TaskStatus::Pending,
        };

        repo.save(&pr.id, &task)?;

        // Verify initial status
        let tasks = repo.find_by_pr(&pr.id)?;
        assert_eq!(tasks[0].status, TaskStatus::Pending);

        // Update status
        repo.update_status(&task.id, TaskStatus::Reviewed)?;

        // Verify updated status
        let tasks = repo.find_by_pr(&pr.id)?;
        assert_eq!(tasks[0].status, TaskStatus::Reviewed);

        Ok(())
    }

    #[test]
    fn test_note_repository_round_trip() -> Result<()> {
        let db = Database::open_at(std::path::PathBuf::from(":memory:"))?;
        let conn = db.connection();
        let note_repo = NoteRepository::new(conn.clone());

        let task_id = "task-note-1".to_string();
        let note = Note {
            task_id: task_id.clone(),
            body: "Body".into(),
            updated_at: "now".into(),
        };

        // Save and fetch single note
        note_repo.save(&note)?;
        let fetched = note_repo.find_by_task(&task_id)?;
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().body, "Body");

        // Fetch via find_by_tasks
        let all = note_repo.find_by_tasks(&[task_id.clone()])?;
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].task_id, task_id);

        // Delete and ensure removal
        note_repo.delete_by_task(&note.task_id)?;
        assert!(note_repo.find_by_task(&note.task_id)?.is_none());

        Ok(())
    }
}
