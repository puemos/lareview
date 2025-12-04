//! Repository traits and implementations for data access

use crate::domain::{Note, PullRequest, PullRequestId, ReviewTask, TaskId};
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

        conn.execute(
            r#"
            INSERT OR REPLACE INTO tasks (id, pull_request_id, title, description, files, stats, insight, patches, diagram, ai_generated)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
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
            ),
        )?;
        Ok(())
    }

    pub fn find_by_pr(&self, pr_id: &PullRequestId) -> Result<Vec<ReviewTask>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, description, files, stats, insight, patches, diagram, ai_generated FROM tasks WHERE pull_request_id = ?1",
        )?;

        let rows = stmt.query_map([pr_id], |row| {
            let files_json: String = row.get(3)?;
            let stats_json: String = row.get(4)?;
            let patches_json: Option<String> = row.get(6)?;

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
            ) = row?;
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
            });
        }
        Ok(tasks)
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
}
