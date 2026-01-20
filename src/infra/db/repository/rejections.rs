use super::DbConn;
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a recorded rejection of feedback (when user marks feedback as "ignored")
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRejection {
    pub id: String,
    pub feedback_id: String,
    pub review_id: String,
    pub rule_id: Option<String>,
    pub agent_id: String,
    pub impact: String,
    pub confidence: f64,
    pub file_extension: Option<String>,
    pub title: String,
    pub created_at: String,
}

/// Statistics about rejection rates for a rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleRejectionStats {
    pub rule_id: String,
    pub total_feedback: i64,
    pub rejected_count: i64,
    pub rejection_rate: f64,
}

/// Statistics about rejection rates for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRejectionStats {
    pub agent_id: String,
    pub total_feedback: i64,
    pub rejected_count: i64,
    pub rejection_rate: f64,
}

pub struct FeedbackRejectionRepository {
    conn: DbConn,
}

impl FeedbackRejectionRepository {
    pub fn new(conn: DbConn) -> Self {
        Self { conn }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_rejection(
        &self,
        feedback_id: &str,
        review_id: &str,
        rule_id: Option<&str>,
        agent_id: &str,
        impact: &str,
        confidence: f64,
        file_extension: Option<&str>,
        title: &str,
    ) -> Result<String> {
        let conn = self
            .conn
            .lock()
            .expect("FeedbackRejectionRepository: failed to acquire database lock");

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO feedback_rejections (
                id, feedback_id, review_id, rule_id, agent_id, impact, confidence,
                file_extension, title, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            rusqlite::params![
                id,
                feedback_id,
                review_id,
                rule_id,
                agent_id,
                impact,
                confidence,
                file_extension,
                title,
                now
            ],
        )?;

        Ok(id)
    }

    /// Check if a rejection already exists for a feedback item
    pub fn rejection_exists(&self, feedback_id: &str) -> Result<bool> {
        let conn = self
            .conn
            .lock()
            .expect("FeedbackRejectionRepository: failed to acquire database lock");

        let exists = conn
            .prepare("SELECT 1 FROM feedback_rejections WHERE feedback_id = ?1")?
            .exists([feedback_id])?;

        Ok(exists)
    }

    /// Get rejection statistics by rule
    pub fn get_rule_stats(&self) -> Result<Vec<RuleRejectionStats>> {
        let conn = self
            .conn
            .lock()
            .expect("FeedbackRejectionRepository: failed to acquire database lock");

        let mut stmt = conn.prepare(
            r#"
            SELECT
                f.rule_id,
                COUNT(*) as total_feedback,
                SUM(CASE WHEN fr.id IS NOT NULL THEN 1 ELSE 0 END) as rejected_count
            FROM feedback f
            LEFT JOIN feedback_rejections fr ON f.id = fr.feedback_id
            WHERE f.rule_id IS NOT NULL
            GROUP BY f.rule_id
            HAVING total_feedback > 0
            ORDER BY rejected_count DESC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            let rule_id: String = row.get(0)?;
            let total: i64 = row.get(1)?;
            let rejected: i64 = row.get(2)?;
            let rate = if total > 0 {
                rejected as f64 / total as f64
            } else {
                0.0
            };

            Ok(RuleRejectionStats {
                rule_id,
                total_feedback: total,
                rejected_count: rejected,
                rejection_rate: rate,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Get rejection statistics by agent
    pub fn get_agent_stats(&self) -> Result<Vec<AgentRejectionStats>> {
        let conn = self
            .conn
            .lock()
            .expect("FeedbackRejectionRepository: failed to acquire database lock");

        // Extract agent_id from author field (format: "agent:agent_id")
        let mut stmt = conn.prepare(
            r#"
            SELECT
                REPLACE(author, 'agent:', '') as agent_id,
                COUNT(*) as total_feedback,
                SUM(CASE WHEN status = 'ignored' THEN 1 ELSE 0 END) as rejected_count
            FROM feedback
            WHERE author LIKE 'agent:%'
            GROUP BY author
            HAVING total_feedback > 0
            ORDER BY total_feedback DESC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            let agent_id: String = row.get(0)?;
            let total: i64 = row.get(1)?;
            let rejected: i64 = row.get(2)?;
            let rate = if total > 0 {
                rejected as f64 / total as f64
            } else {
                0.0
            };

            Ok(AgentRejectionStats {
                agent_id,
                total_feedback: total,
                rejected_count: rejected,
                rejection_rate: rate,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Get all rejections for analysis
    pub fn list_all(&self, limit: usize) -> Result<Vec<FeedbackRejection>> {
        let conn = self
            .conn
            .lock()
            .expect("FeedbackRejectionRepository: failed to acquire database lock");

        let mut stmt = conn.prepare(
            r#"
            SELECT id, feedback_id, review_id, rule_id, agent_id, impact, confidence,
                   file_extension, title, created_at
            FROM feedback_rejections
            ORDER BY created_at DESC
            LIMIT ?1
            "#,
        )?;

        let rows = stmt.query_map([limit as i64], |row| {
            Ok(FeedbackRejection {
                id: row.get(0)?,
                feedback_id: row.get(1)?,
                review_id: row.get(2)?,
                rule_id: row.get(3)?,
                agent_id: row.get(4)?,
                impact: row.get(5)?,
                confidence: row.get(6)?,
                file_extension: row.get(7)?,
                title: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    // --- Learning compaction methods ---

    /// Count rejections since last compaction (unprocessed)
    pub fn get_unprocessed_count(&self) -> Result<i64> {
        let conn = self
            .conn
            .lock()
            .expect("FeedbackRejectionRepository: failed to acquire database lock");

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM feedback_rejections WHERE processed_for_learning = 0",
            [],
            |row| row.get(0),
        )?;

        Ok(count)
    }

    /// Check if compaction should trigger based on threshold
    pub fn should_trigger_compaction(&self, threshold: i64) -> Result<bool> {
        let count = self.get_unprocessed_count()?;
        Ok(count >= threshold)
    }

    /// Get unprocessed rejections for learning agent
    pub fn get_unprocessed_rejections(&self, limit: usize) -> Result<Vec<FeedbackRejection>> {
        let conn = self
            .conn
            .lock()
            .expect("FeedbackRejectionRepository: failed to acquire database lock");

        let mut stmt = conn.prepare(
            r#"
            SELECT id, feedback_id, review_id, rule_id, agent_id, impact, confidence,
                   file_extension, title, created_at
            FROM feedback_rejections
            WHERE processed_for_learning = 0
            ORDER BY created_at ASC
            LIMIT ?1
            "#,
        )?;

        let rows = stmt.query_map([limit as i64], |row| {
            Ok(FeedbackRejection {
                id: row.get(0)?,
                feedback_id: row.get(1)?,
                review_id: row.get(2)?,
                rule_id: row.get(3)?,
                agent_id: row.get(4)?,
                impact: row.get(5)?,
                confidence: row.get(6)?,
                file_extension: row.get(7)?,
                title: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Mark rejections as processed for learning
    pub fn mark_processed(&self, rejection_ids: &[String]) -> Result<usize> {
        if rejection_ids.is_empty() {
            return Ok(0);
        }

        let conn = self
            .conn
            .lock()
            .expect("FeedbackRejectionRepository: failed to acquire database lock");

        let placeholders: String = rejection_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "UPDATE feedback_rejections SET processed_for_learning = 1 WHERE id IN ({placeholders})"
        );

        let params: Vec<&dyn rusqlite::ToSql> = rejection_ids
            .iter()
            .map(|id| id as &dyn rusqlite::ToSql)
            .collect();

        let rows = conn.execute(&sql, params.as_slice())?;
        Ok(rows)
    }
}
