//! Repository for MergeConfidence persistence

use crate::domain::MergeConfidence;
use anyhow::{Context, Result};
use rusqlite::params;
use uuid::Uuid;

use super::{DbConn, Repository};

pub struct MergeConfidenceRepository {
    conn: DbConn,
}

impl Repository for MergeConfidenceRepository {}

impl MergeConfidenceRepository {
    pub fn new(conn: DbConn) -> Self {
        Self { conn }
    }

    /// Save or update merge confidence for a run (upsert)
    pub fn save(&self, run_id: &str, confidence: &MergeConfidence) -> Result<()> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");

        let reasons_json =
            serde_json::to_string(&confidence.reasons).context("serialize reasons")?;

        // Try to find existing record
        let existing_id: Option<String> = conn
            .query_row(
                "SELECT id FROM merge_confidence WHERE run_id = ?1",
                [run_id],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing_id {
            // Update existing
            conn.execute(
                r#"
                UPDATE merge_confidence SET
                    score = ?1,
                    reasons = ?2,
                    computed_at = ?3
                WHERE id = ?4
                "#,
                params![confidence.score, reasons_json, confidence.computed_at, id,],
            )
            .context("update merge confidence")?;
        } else {
            // Insert new
            let id = Uuid::new_v4().to_string();
            conn.execute(
                r#"
                INSERT INTO merge_confidence
                    (id, run_id, score, reasons, computed_at)
                VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                params![id, run_id, confidence.score, reasons_json, confidence.computed_at,],
            )
            .context("insert merge confidence")?;
        }

        Ok(())
    }

    /// Find merge confidence by run ID
    pub fn find_by_run_id(&self, run_id: &str) -> Result<Option<MergeConfidence>> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        let mut stmt = conn.prepare(
            r#"
            SELECT score, reasons, computed_at
            FROM merge_confidence
            WHERE run_id = ?1
            "#,
        )?;

        let result = stmt.query_row([run_id], |row| {
            let score: f64 = row.get(0)?;
            let reasons_json: String = row.get(1)?;
            let computed_at: String = row.get(2)?;

            Ok((score, reasons_json, computed_at))
        });

        match result {
            Ok((score, reasons_json, computed_at)) => {
                let reasons: Vec<String> =
                    serde_json::from_str(&reasons_json).unwrap_or_default();

                Ok(Some(MergeConfidence {
                    score: score as f32,
                    reasons,
                    computed_at,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Delete merge confidence for a run
    pub fn delete_by_run_id(&self, run_id: &str) -> Result<usize> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        let affected = conn.execute("DELETE FROM merge_confidence WHERE run_id = ?1", [run_id])?;
        Ok(affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};

    fn setup_test_db() -> DbConn {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE review_runs (
                id TEXT PRIMARY KEY,
                review_id TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                input_ref TEXT NOT NULL,
                diff_text TEXT NOT NULL,
                diff_hash TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'completed',
                created_at TEXT NOT NULL
            );

            CREATE TABLE merge_confidence (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL UNIQUE,
                score REAL NOT NULL CHECK (score >= 1.0 AND score <= 5.0),
                reasons TEXT NOT NULL,
                computed_at TEXT NOT NULL,
                FOREIGN KEY(run_id) REFERENCES review_runs(id) ON DELETE CASCADE
            );

            INSERT INTO review_runs (id, review_id, agent_id, input_ref, diff_text, diff_hash, created_at)
            VALUES ('run-1', 'rev-1', 'agent-1', 'input', 'diff', 'hash', '2024-01-01');
            "#,
        )
        .unwrap();
        Arc::new(Mutex::new(conn))
    }

    #[test]
    fn test_save_and_find() {
        let conn = setup_test_db();
        let repo = MergeConfidenceRepository::new(conn);

        let confidence = MergeConfidence::new(
            4.0,
            vec![
                "✓ Well-scoped changes".to_string(),
                "✓ Good test coverage".to_string(),
            ],
        );

        repo.save("run-1", &confidence).unwrap();

        let found = repo.find_by_run_id("run-1").unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.score, 4.0);
        assert_eq!(found.reasons.len(), 2);
    }

    #[test]
    fn test_upsert() {
        let conn = setup_test_db();
        let repo = MergeConfidenceRepository::new(conn);

        let confidence1 = MergeConfidence::new(3.0, vec!["Initial assessment".to_string()]);
        repo.save("run-1", &confidence1).unwrap();

        let confidence2 = MergeConfidence::new(
            5.0,
            vec![
                "✓ All issues addressed".to_string(),
                "✓ Tests passing".to_string(),
            ],
        );
        repo.save("run-1", &confidence2).unwrap();

        let found = repo.find_by_run_id("run-1").unwrap().unwrap();
        assert_eq!(found.score, 5.0);
        assert_eq!(found.reasons.len(), 2);
    }

    #[test]
    fn test_delete() {
        let conn = setup_test_db();
        let repo = MergeConfidenceRepository::new(conn);

        let confidence = MergeConfidence::default();
        repo.save("run-1", &confidence).unwrap();

        let deleted = repo.delete_by_run_id("run-1").unwrap();
        assert_eq!(deleted, 1);

        let found = repo.find_by_run_id("run-1").unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_fractional_score() {
        let conn = setup_test_db();
        let repo = MergeConfidenceRepository::new(conn);

        let confidence = MergeConfidence::new(4.5, vec!["Good but minor concerns".to_string()]);
        repo.save("run-1", &confidence).unwrap();

        let found = repo.find_by_run_id("run-1").unwrap().unwrap();
        assert_eq!(found.score, 4.5);
    }
}
