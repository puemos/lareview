use super::DbConn;
use crate::domain::{ReviewRun, ReviewRunId};
use anyhow::Result;

/// Repository for review run operations.
pub struct ReviewRunRepository {
    conn: DbConn,
}

impl ReviewRunRepository {
    pub fn new(conn: DbConn) -> Self {
        Self { conn }
    }

    pub fn save(&self, run: &ReviewRun) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT OR REPLACE INTO review_runs (id, review_id, agent_id, input_ref, diff_text, diff_hash, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            (
                &run.id,
                &run.review_id,
                &run.agent_id,
                &run.input_ref,
                &run.diff_text,
                &run.diff_hash,
                &run.created_at,
            ),
        )?;
        Ok(())
    }

    pub fn list_all(&self) -> Result<Vec<ReviewRun>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, review_id, agent_id, input_ref, diff_text, diff_hash, created_at FROM review_runs ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(ReviewRun {
                id: row.get::<_, ReviewRunId>(0)?,
                review_id: row.get(1)?,
                agent_id: row.get(2)?,
                input_ref: row.get(3)?,
                diff_text: row.get(4)?,
                diff_hash: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}
