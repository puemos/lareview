use super::DbConn;
use crate::domain::{
    Feedback, FeedbackAnchor, FeedbackImpact, FeedbackSide, HunkRef, ReviewStatus,
};
use anyhow::Result;
use chrono::Utc;
use rusqlite::Row;

use std::str::FromStr;

pub struct FeedbackRepository {
    conn: DbConn,
}

impl FeedbackRepository {
    pub fn new(conn: DbConn) -> Self {
        Self { conn }
    }

    pub fn save(&self, feedback: &Feedback) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let anchor = feedback.anchor.as_ref();
        let hunk_ref = anchor
            .and_then(|a| a.hunk_ref.as_ref())
            .map(|h| serde_json::to_string(h).unwrap_or_default());

        conn.execute(
            r#"
            INSERT OR REPLACE INTO feedback (
                id, review_id, task_id, title, status, impact,
                anchor_file_path, anchor_line, anchor_side, anchor_hunk_ref, anchor_head_sha,
                author, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            "#,
            rusqlite::params![
                feedback.id,
                feedback.review_id,
                feedback.task_id,
                feedback.title,
                feedback.status.to_string(),
                feedback.impact.to_string(),
                anchor.and_then(|a| a.file_path.clone()),
                anchor.and_then(|a| a.line_number.map(|n| n as i32)),
                anchor.and_then(|a| a.side).map(|s| match s {
                    FeedbackSide::Old => "old",
                    FeedbackSide::New => "new",
                }),
                hunk_ref,
                anchor.and_then(|a| a.head_sha.clone()),
                feedback.author,
                feedback.created_at,
                feedback.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn update_status(&self, id: &str, status: ReviewStatus) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let updated = conn.execute(
            "UPDATE feedback SET status = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![id, status.to_string(), Utc::now().to_rfc3339()],
        )?;
        Ok(updated)
    }

    pub fn update_impact(&self, id: &str, impact: FeedbackImpact) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let updated = conn.execute(
            "UPDATE feedback SET impact = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![id, impact.to_string(), Utc::now().to_rfc3339()],
        )?;
        Ok(updated)
    }

    pub fn update_title(&self, id: &str, title: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let updated = conn.execute(
            "UPDATE feedback SET title = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![id, title, Utc::now().to_rfc3339()],
        )?;
        Ok(updated)
    }

    pub fn touch(&self, id: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let updated = conn.execute(
            "UPDATE feedback SET updated_at = ?2 WHERE id = ?1",
            rusqlite::params![id, Utc::now().to_rfc3339()],
        )?;
        Ok(updated)
    }

    pub fn find_by_id(&self, id: &str) -> Result<Option<Feedback>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, review_id, task_id, title, status, impact,
                   anchor_file_path, anchor_line, anchor_side, anchor_hunk_ref, anchor_head_sha,
                   author, created_at, updated_at
            FROM feedback
            WHERE id = ?1
            "#,
        )?;

        let mut rows = stmt.query_map([id], Self::row_to_feedback)?;
        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    pub fn find_by_review(&self, review_id: &str) -> Result<Vec<Feedback>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, review_id, task_id, title, status, impact,
                   anchor_file_path, anchor_line, anchor_side, anchor_hunk_ref, anchor_head_sha,
                   author, created_at, updated_at
            FROM feedback
            WHERE review_id = ?1
            ORDER BY anchor_file_path, anchor_line, updated_at DESC
            "#,
        )?;

        let rows = stmt.query_map([review_id], Self::row_to_feedback)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn delete_by_review(&self, review_id: &str) -> Result<usize> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let count = tx.execute("DELETE FROM feedback WHERE review_id = ?", [review_id])?;
        tx.commit()?;
        Ok(count)
    }

    pub fn delete(&self, id: &str) -> Result<usize> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let count = tx.execute("DELETE FROM feedback WHERE id = ?", [id])?;
        tx.commit()?;
        Ok(count)
    }

    fn row_to_feedback(row: &Row) -> rusqlite::Result<Feedback> {
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

            Some(FeedbackAnchor {
                file_path: anchor_file_path,
                line_number: anchor_line.map(|n| n as u32),
                side: anchor_side.as_deref().and_then(|s| {
                    if s == "old" {
                        Some(FeedbackSide::Old)
                    } else if s == "new" {
                        Some(FeedbackSide::New)
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

        Ok(Feedback {
            id: row.get(0)?,
            review_id: row.get(1)?,
            task_id: row.get(2)?,
            title: row.get(3)?,
            status: ReviewStatus::from_str(&status).unwrap_or_default(),
            impact: FeedbackImpact::from_str(&impact).unwrap_or_default(),
            anchor,
            author: row.get(11)?,
            created_at: row.get(12)?,
            updated_at: row.get(13)?,
        })
    }
}
