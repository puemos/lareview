use super::DbConn;
use crate::domain::PullRequest;
use anyhow::Result;

/// Repository for pull request operations.
pub struct PullRequestRepository {
    conn: DbConn,
}

impl PullRequestRepository {
    pub fn new(conn: DbConn) -> Self {
        Self { conn }
    }

    pub fn save(&self, pr: &PullRequest) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT OR REPLACE INTO pull_requests (id, title, description, repo, author, branch, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            (
                &pr.id,
                &pr.title,
                &pr.description,
                &pr.repo,
                &pr.author,
                &pr.branch,
                &pr.created_at,
            ),
        )?;
        Ok(())
    }

    pub fn list_all(&self) -> Result<Vec<PullRequest>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, description, repo, author, branch, created_at FROM pull_requests ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(PullRequest {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                repo: row.get(3)?,
                author: row.get(4)?,
                branch: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}
