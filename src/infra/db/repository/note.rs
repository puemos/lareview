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
            "INSERT OR REPLACE INTO notes (task_id, file_path, line_number, body, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                &note.task_id,
                &note.file_path,
                note.line_number.map(|n| n as i32),
                &note.body,
                &note.updated_at,
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

    pub fn find_by_task(&self, task_id: &TaskId) -> Result<Option<Note>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT task_id, file_path, line_number, body, updated_at FROM notes WHERE task_id = ?1 AND file_path IS NULL AND line_number IS NULL",
        )?;

        let mut rows = stmt.query([task_id])?;
        if let Some(row) = rows.next()? {
            let task_id_val: String = row.get(0)?;
            let file_path_val: Option<String> = row.get(1)?;
            let line_number_val: Option<i32> = row.get(2)?;
            let body_val: String = row.get(3)?;
            let updated_at_val: String = row.get(4)?;

            Ok(Some(Note {
                task_id: task_id_val,
                file_path: file_path_val,
                line_number: line_number_val.map(|n| n as u32),
                body: body_val,
                updated_at: updated_at_val,
            }))
        } else {
            Ok(None)
        }
    }

    /// Find all line-specific notes for a task.
    #[allow(dead_code)]
    pub fn find_line_notes_for_task(&self, task_id: &TaskId) -> Result<Vec<Note>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT task_id, file_path, line_number, body, updated_at FROM notes WHERE task_id = ?1 AND file_path IS NOT NULL AND line_number IS NOT NULL ORDER BY file_path, line_number",
        )?;

        let rows = stmt.query_map([task_id], |row| {
            let task_id_val: String = row.get(0)?;
            let file_path_val: Option<String> = row.get(1)?;
            let line_number_val: Option<i32> = row.get(2)?;
            let body_val: String = row.get(3)?;
            let updated_at_val: String = row.get(4)?;

            Ok(Note {
                task_id: task_id_val,
                file_path: file_path_val,
                line_number: line_number_val.map(|n| n as u32),
                body: body_val,
                updated_at: updated_at_val,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Find a specific line note by task, file, and line number.
    #[allow(dead_code)]
    pub fn find_line_note(
        &self,
        task_id: &TaskId,
        file_path: &str,
        line_number: u32,
    ) -> Result<Option<Note>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT task_id, file_path, line_number, body, updated_at FROM notes WHERE task_id = ?1 AND file_path = ?2 AND line_number = ?3",
        )?;

        let mut rows = stmt.query(rusqlite::params![task_id, file_path, &(line_number as i32)])?;
        if let Some(row) = rows.next()? {
            let task_id_val: String = row.get(0)?;
            let file_path_val: Option<String> = row.get(1)?;
            let line_number_val: Option<i32> = row.get(2)?;
            let body_val: String = row.get(3)?;
            let updated_at_val: String = row.get(4)?;

            Ok(Some(Note {
                task_id: task_id_val,
                file_path: file_path_val,
                line_number: line_number_val.map(|n| n as u32),
                body: body_val,
                updated_at: updated_at_val,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn find_all_for_tasks(&self, task_ids: &[TaskId]) -> Result<Vec<Note>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT task_id, file_path, line_number, body, updated_at FROM notes WHERE task_id = ?1 ORDER BY task_id, file_path, line_number",
        )?;

        let mut all_notes = Vec::new();
        for id in task_ids {
            let rows = stmt.query_map([id], |row| {
                let task_id_val: String = row.get(0)?;
                let file_path_val: Option<String> = row.get(1)?;
                let line_number_val: Option<i32> = row.get(2)?;
                let body_val: String = row.get(3)?;
                let updated_at_val: String = row.get(4)?;

                Ok(Note {
                    task_id: task_id_val,
                    file_path: file_path_val,
                    line_number: line_number_val.map(|n| n as u32),
                    body: body_val,
                    updated_at: updated_at_val,
                })
            })?;
            for note in rows {
                all_notes.push(note?);
            }
        }

        Ok(all_notes)
    }
}
