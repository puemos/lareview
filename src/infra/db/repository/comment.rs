use super::DbConn;
use crate::domain::Comment;
use anyhow::Result;
use chrono::Utc;
use rusqlite::Row;

pub struct CommentRepository {
    conn: DbConn,
}

impl CommentRepository {
    pub fn new(conn: DbConn) -> Self {
        Self { conn }
    }

    pub fn save(&self, comment: &Comment) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .expect("CommentRepository: failed to acquire database lock");
        conn.execute(
            r#"
            INSERT OR REPLACE INTO comments (
                id, feedback_id, author, body, parent_id, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            rusqlite::params![
                comment.id,
                comment.feedback_id,
                comment.author,
                comment.body,
                comment.parent_id,
                comment.created_at,
                comment.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn list_for_feedback(&self, feedback_id: &str) -> Result<Vec<Comment>> {
        let conn = self
            .conn
            .lock()
            .expect("CommentRepository: failed to acquire database lock");
        let mut stmt = conn.prepare(
            r#"
            SELECT id, feedback_id, author, body, parent_id, created_at, updated_at
            FROM comments
            WHERE feedback_id = ?1
            ORDER BY created_at
            "#,
        )?;

        let rows = stmt.query_map([feedback_id], Self::row_to_comment)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn delete_by_feedback(&self, feedback_id: &str) -> Result<usize> {
        let conn = self
            .conn
            .lock()
            .expect("CommentRepository: failed to acquire database lock");
        let affected =
            conn.execute("DELETE FROM comments WHERE feedback_id = ?1", [feedback_id])?;
        Ok(affected)
    }

    pub fn delete(&self, id: &str) -> Result<usize> {
        let mut conn = self
            .conn
            .lock()
            .expect("CommentRepository: failed to acquire database lock");
        let tx = conn.transaction()?;
        let count = tx.execute("DELETE FROM comments WHERE id = ?", [id])?;
        tx.commit()?;
        Ok(count)
    }

    pub fn touch(&self, id: &str) -> Result<usize> {
        let conn = self
            .conn
            .lock()
            .expect("CommentRepository: failed to acquire database lock");
        let updated = conn.execute(
            "UPDATE comments SET updated_at = ?2 WHERE id = ?1",
            rusqlite::params![id, Utc::now().to_rfc3339()],
        )?;
        Ok(updated)
    }

    fn row_to_comment(row: &Row) -> rusqlite::Result<Comment> {
        Ok(Comment {
            id: row.get(0)?,
            feedback_id: row.get(1)?,
            author: row.get(2)?,
            body: row.get(3)?,
            parent_id: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    }
}
