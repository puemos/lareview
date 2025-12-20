use super::DbConn;
use crate::domain::{Note, TaskId};
use anyhow::Result;

/// Repository for note operations.
pub struct NoteRepository {
    conn: DbConn,
}

impl NoteRepository {
    pub fn new(conn: DbConn) -> Self {
        Self { conn }
    }

    pub fn save(&self, note: &Note) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"INSERT OR REPLACE INTO notes (
                id, task_id, author, body, created_at, updated_at, 
                file_path, line_number, parent_id, root_id, status, title, severity
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)"#,
            rusqlite::params![
                &note.id,
                &note.task_id,
                &note.author,
                &note.body,
                &note.created_at,
                &note.updated_at,
                &note.file_path,
                note.line_number.map(|n| n as i32),
                &note.parent_id,
                &note.root_id,
                match note.status {
                    crate::domain::NoteStatus::Open => "open",
                    crate::domain::NoteStatus::Resolved => "resolved",
                },
                &note.title,
                note.severity.as_ref().map(|s| match s {
                    crate::domain::NoteSeverity::Blocking => "blocking",
                    crate::domain::NoteSeverity::NonBlocking => "non-blocking",
                }),
            ],
        )?;
        Ok(())
    }

    pub fn delete_by_task(&self, task_id: &TaskId) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM notes WHERE task_id = ?1", [task_id])?;
        Ok(affected)
    }

    pub fn delete_by_task_ids(&self, task_ids: &[TaskId]) -> Result<usize> {
        let mut affected_total = 0usize;
        for id in task_ids {
            affected_total += self.delete_by_task(id)?;
        }
        Ok(affected_total)
    }

    fn row_to_note(row: &rusqlite::Row) -> rusqlite::Result<Note> {
        let status_str: String = row.get(10)?;
        let status = if status_str == "resolved" {
            crate::domain::NoteStatus::Resolved
        } else {
            crate::domain::NoteStatus::Open
        };

        let severity_str: Option<String> = row.get(12)?;
        let severity = severity_str.map(|s| {
            if s == "blocking" {
                crate::domain::NoteSeverity::Blocking
            } else {
                crate::domain::NoteSeverity::NonBlocking
            }
        });

        Ok(Note {
            id: row.get(0)?,
            task_id: row.get(1)?,
            author: row.get(2)?,
            body: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            file_path: row.get(6)?,
            line_number: row.get::<_, Option<i32>>(7)?.map(|n| n as u32),
            parent_id: row.get(8)?,
            root_id: row.get(9)?,
            status,
            title: row.get(11)?,
            severity,
        })
    }

    pub fn find_by_task(&self, task_id: &TaskId) -> Result<Option<Note>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, author, body, created_at, updated_at, file_path, line_number, parent_id, root_id, status, title, severity FROM notes WHERE task_id = ?1 AND file_path IS NULL AND line_number IS NULL",
        )?;

        let mut rows = stmt.query([task_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::row_to_note(row)?))
        } else {
            Ok(None)
        }
    }

    /// Find all line-specific notes for a task.
    pub fn find_line_notes_for_task(&self, task_id: &TaskId) -> Result<Vec<Note>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, author, body, created_at, updated_at, file_path, line_number, parent_id, root_id, status, title, severity FROM notes WHERE task_id = ?1 AND file_path IS NOT NULL AND line_number IS NOT NULL ORDER BY file_path, line_number, created_at",
        )?;

        let rows = stmt.query_map([task_id], Self::row_to_note)?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn find_all_for_tasks(&self, task_ids: &[TaskId]) -> Result<Vec<Note>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, author, body, created_at, updated_at, file_path, line_number, parent_id, root_id, status, title, severity FROM notes WHERE task_id = ?1 ORDER BY task_id, file_path, line_number, created_at",
        )?;

        let mut all_notes = Vec::new();
        for id in task_ids {
            let rows = stmt.query_map([id], Self::row_to_note)?;
            for note in rows {
                all_notes.push(note?);
            }
        }

        Ok(all_notes)
    }

    pub fn resolve_thread(&self, _task_id: &TaskId, root_id: &String) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE notes SET status = 'resolved' WHERE root_id = ?1 OR id = ?1",
            [root_id],
        )?;
        Ok(())
    }

    pub fn update_metadata(
        &self,
        note_id: &str,
        title: Option<String>,
        severity: Option<crate::domain::NoteSeverity>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let severity_str = severity.map(|s| match s {
            crate::domain::NoteSeverity::Blocking => "blocking",
            crate::domain::NoteSeverity::NonBlocking => "non-blocking",
        });

        conn.execute(
            "UPDATE notes SET title = COALESCE(?2, title), severity = COALESCE(?3, severity), updated_at = ?4 WHERE id = ?1",
            rusqlite::params![
                note_id,
                title,
                severity_str,
                chrono::Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }
}
