use super::DbConn;
use crate::domain::FeedbackLink;
use anyhow::Result;
use rusqlite::Row;

pub struct FeedbackLinkRepository {
    conn: DbConn,
}

impl FeedbackLinkRepository {
    pub fn new(conn: DbConn) -> Self {
        Self { conn }
    }

    pub fn save(&self, link: &FeedbackLink) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT OR REPLACE INTO feedback_links (
                id, feedback_id, provider, provider_feedback_id, provider_root_comment_id, last_synced_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            rusqlite::params![
                link.id,
                link.feedback_id,
                link.provider,
                link.provider_feedback_id,
                link.provider_root_comment_id,
                link.last_synced_at,
            ],
        )?;
        Ok(())
    }

    pub fn find_by_feedback(&self, feedback_id: &str) -> Result<Option<FeedbackLink>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, feedback_id, provider, provider_feedback_id, provider_root_comment_id, last_synced_at
            FROM feedback_links
            WHERE feedback_id = ?1
            "#,
        )?;

        let mut rows = stmt.query_map([feedback_id], Self::row_to_link)?;
        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    pub fn find_by_feedback_ids(&self, ids: &[String]) -> Result<Vec<FeedbackLink>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut links = Vec::new();
        for id in ids {
            if let Some(link) = self.find_by_feedback(id)? {
                links.push(link);
            }
        }
        Ok(links)
    }

    fn row_to_link(row: &Row) -> rusqlite::Result<FeedbackLink> {
        Ok(FeedbackLink {
            id: row.get(0)?,
            feedback_id: row.get(1)?,
            provider: row.get(2)?,
            provider_feedback_id: row.get(3)?,
            provider_root_comment_id: row.get(4)?,
            last_synced_at: row.get(5)?,
        })
    }
}
