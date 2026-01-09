use super::DbConn;
use crate::domain::{ReviewId, ReviewRun, ReviewRunId, ReviewRunStatus};
use anyhow::Result;
use std::str::FromStr;

/// Repository for review run operations.
pub struct ReviewRunRepository {
    conn: DbConn,
}

impl ReviewRunRepository {
    pub fn new(conn: DbConn) -> Self {
        Self { conn }
    }
}

impl ReviewRunRepository {
    pub fn save(&self, run: &ReviewRun) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .expect("ReviewRunRepository: failed to acquire database lock");
        conn.execute(
            r#"
            INSERT INTO review_runs (id, review_id, agent_id, input_ref, diff_text, diff_hash, status, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(id) DO NOTHING
            "#,
            (
                &run.id,
                &run.review_id,
                &run.agent_id,
                &run.input_ref,
                &run.diff_text,
                &run.diff_hash,
                &run.status.to_string(),
                &run.created_at,
            ),
        )?;
        Ok(())
    }

    pub fn find_by_id(&self, id: &ReviewRunId) -> Result<Option<ReviewRun>> {
        let conn = self
            .conn
            .lock()
            .expect("ReviewRunRepository: failed to acquire database lock");
        let mut stmt = conn.prepare(
            "SELECT id, review_id, agent_id, input_ref, diff_text, diff_hash, status, created_at FROM review_runs WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map([id], |row| {
            let status_str: String = row.get(6)?;
            let status = ReviewRunStatus::from_str(&status_str).unwrap_or_default();
            Ok(ReviewRun {
                id: row.get::<_, ReviewRunId>(0)?,
                review_id: row.get(1)?,
                agent_id: row.get(2)?,
                input_ref: row.get(3)?,
                diff_text: row.get(4)?,
                diff_hash: row.get(5)?,
                status,
                created_at: row.get(7)?,
            })
        })?;

        match rows.next() {
            Some(row) => row.map(Some).map_err(Into::into),
            None => Ok(None),
        }
    }

    pub fn find_by_review_id(&self, review_id: &ReviewId) -> Result<Vec<ReviewRun>> {
        let conn = self
            .conn
            .lock()
            .expect("ReviewRunRepository: failed to acquire database lock");
        let mut stmt = conn.prepare(
            "SELECT id, review_id, agent_id, input_ref, diff_text, diff_hash, status, created_at FROM review_runs WHERE review_id = ?1",
        )?;
        let rows = stmt.query_map([review_id], |row| {
            let status_str: String = row.get(6)?;
            let status = ReviewRunStatus::from_str(&status_str).unwrap_or_default();
            Ok(ReviewRun {
                id: row.get::<_, ReviewRunId>(0)?,
                review_id: row.get(1)?,
                agent_id: row.get(2)?,
                input_ref: row.get(3)?,
                diff_text: row.get(4)?,
                diff_hash: row.get(5)?,
                status,
                created_at: row.get(7)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn update_status(&self, id: &ReviewRunId, status: ReviewRunStatus) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .expect("ReviewRunRepository: failed to acquire database lock");
        conn.execute(
            "UPDATE review_runs SET status = ?1 WHERE id = ?2",
            (&status.to_string(), id),
        )?;
        Ok(())
    }

    pub fn delete_by_review_id(&self, review_id: &ReviewId) -> Result<usize> {
        let conn = self
            .conn
            .lock()
            .expect("ReviewRunRepository: failed to acquire database lock");
        let affected = conn.execute("DELETE FROM review_runs WHERE review_id = ?1", [review_id])?;
        Ok(affected)
    }

    pub fn list_all(&self) -> Result<Vec<ReviewRun>> {
        let conn = self
            .conn
            .lock()
            .expect("ReviewRunRepository: failed to acquire database lock");
        let mut stmt = conn.prepare(
            "SELECT id, review_id, agent_id, input_ref, diff_text, diff_hash, status, created_at FROM review_runs ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            let status_str: String = row.get(6)?;
            let status = ReviewRunStatus::from_str(&status_str).unwrap_or_default();
            Ok(ReviewRun {
                id: row.get::<_, ReviewRunId>(0)?,
                review_id: row.get(1)?,
                agent_id: row.get(2)?,
                input_ref: row.get(3)?,
                diff_text: row.get(4)?,
                diff_hash: row.get(5)?,
                status,
                created_at: row.get(7)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}
