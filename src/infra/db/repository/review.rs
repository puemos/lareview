use super::DbConn;
use crate::domain::{Review, ReviewId, ReviewRunId, ReviewSource};
use anyhow::Result;

/// Repository for review operations.
pub struct ReviewRepository {
    conn: DbConn,
}

impl ReviewRepository {
    pub fn new(conn: DbConn) -> Self {
        Self { conn }
    }

    pub fn save(&self, review: &Review) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let source_json = serde_json::to_string(&review.source)?;
        conn.execute(
            r#"
            INSERT OR REPLACE INTO reviews (id, title, summary, source_json, active_run_id, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            (
                &review.id,
                &review.title,
                &review.summary,
                &source_json,
                &review.active_run_id,
                &review.created_at,
                &review.updated_at,
            ),
        )?;
        Ok(())
    }

    pub fn list_all(&self) -> Result<Vec<Review>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, summary, source_json, active_run_id, created_at, updated_at FROM reviews ORDER BY updated_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            let source_json: String = row.get(3)?;
            let source: ReviewSource =
                serde_json::from_str(&source_json).unwrap_or(ReviewSource::DiffPaste {
                    diff_hash: String::new(),
                });
            Ok(Review {
                id: row.get::<_, ReviewId>(0)?,
                title: row.get(1)?,
                summary: row.get(2)?,
                source,
                active_run_id: row.get::<_, Option<ReviewRunId>>(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}
