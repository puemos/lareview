use super::DbConn;
use crate::domain::{HunkRef, ReviewStatus, Thread, ThreadAnchor, ThreadImpact, ThreadSide};
use anyhow::Result;
use chrono::Utc;
use rusqlite::Row;

use std::str::FromStr;

pub struct ThreadRepository {
    conn: DbConn,
}

impl ThreadRepository {
    pub fn new(conn: DbConn) -> Self {
        Self { conn }
    }

    pub fn save(&self, thread: &Thread) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let anchor = thread.anchor.as_ref();
        let hunk_ref = anchor
            .and_then(|a| a.hunk_ref.as_ref())
            .map(|h| serde_json::to_string(h).unwrap_or_default());

        conn.execute(
            r#"
            INSERT OR REPLACE INTO threads (
                id, review_id, task_id, title, status, impact,
                anchor_file_path, anchor_line, anchor_side, anchor_hunk_ref, anchor_head_sha,
                author, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            "#,
            rusqlite::params![
                thread.id,
                thread.review_id,
                thread.task_id,
                thread.title,
                thread.status.to_string(),
                thread.impact.to_string(),
                anchor.and_then(|a| a.file_path.clone()),
                anchor.and_then(|a| a.line_number.map(|n| n as i32)),
                anchor.and_then(|a| a.side).map(|s| match s {
                    ThreadSide::Old => "old",
                    ThreadSide::New => "new",
                }),
                hunk_ref,
                anchor.and_then(|a| a.head_sha.clone()),
                thread.author,
                thread.created_at,
                thread.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn update_status(&self, id: &str, status: ReviewStatus) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let updated = conn.execute(
            "UPDATE threads SET status = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![id, status.to_string(), Utc::now().to_rfc3339()],
        )?;
        Ok(updated)
    }

    pub fn update_impact(&self, id: &str, impact: ThreadImpact) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let updated = conn.execute(
            "UPDATE threads SET impact = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![id, impact.to_string(), Utc::now().to_rfc3339()],
        )?;
        Ok(updated)
    }

    pub fn update_title(&self, id: &str, title: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let updated = conn.execute(
            "UPDATE threads SET title = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![id, title, Utc::now().to_rfc3339()],
        )?;
        Ok(updated)
    }

    pub fn touch(&self, id: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let updated = conn.execute(
            "UPDATE threads SET updated_at = ?2 WHERE id = ?1",
            rusqlite::params![id, Utc::now().to_rfc3339()],
        )?;
        Ok(updated)
    }

    pub fn find_by_review(&self, review_id: &str) -> Result<Vec<Thread>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, review_id, task_id, title, status, impact,
                   anchor_file_path, anchor_line, anchor_side, anchor_hunk_ref, anchor_head_sha,
                   author, created_at, updated_at
            FROM threads
            WHERE review_id = ?1
            ORDER BY anchor_file_path, anchor_line, updated_at DESC
            "#,
        )?;

        let rows = stmt.query_map([review_id], Self::row_to_thread)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn delete_by_review(&self, review_id: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM threads WHERE review_id = ?1", [review_id])?;
        Ok(affected)
    }

    fn row_to_thread(row: &Row) -> rusqlite::Result<Thread> {
        let status: String = row.get(4)?;
        let impact: String = row.get(5)?;
        let anchor_file_path: Option<String> = row.get(6)?;
        let anchor_line: Option<i32> = row.get(7)?;
        let anchor_side: Option<String> = row.get(8)?;
        let anchor_hunk_ref: Option<String> = row.get(9)?;
        let anchor_head_sha: Option<String> = row.get(10)?;

        let anchor = if anchor_file_path.is_some()
            || anchor_line.is_some()
            || anchor_side.is_some()
            || anchor_hunk_ref.is_some()
            || anchor_head_sha.is_some()
        {
            let hunk_ref: Option<HunkRef> = anchor_hunk_ref
                .as_deref()
                .and_then(|json| serde_json::from_str(json).ok());

            Some(ThreadAnchor {
                file_path: anchor_file_path,
                line_number: anchor_line.map(|n| n as u32),
                side: anchor_side.as_deref().and_then(|s| {
                    if s == "old" {
                        Some(ThreadSide::Old)
                    } else if s == "new" {
                        Some(ThreadSide::New)
                    } else {
                        None
                    }
                }),
                hunk_ref,
                head_sha: anchor_head_sha,
            })
        } else {
            None
        };

        Ok(Thread {
            id: row.get(0)?,
            review_id: row.get(1)?,
            task_id: row.get(2)?,
            title: row.get(3)?,
            status: ReviewStatus::from_str(&status).unwrap_or_default(),
            impact: ThreadImpact::from_str(&impact).unwrap_or_default(),
            anchor,
            author: row.get(11)?,
            created_at: row.get(12)?,
            updated_at: row.get(13)?,
        })
    }
}
