use super::DbConn;
use crate::domain::LinkedRepo;
use anyhow::Result;

pub struct RepoRepository {
    conn: DbConn,
}

impl RepoRepository {
    pub fn new(conn: DbConn) -> Self {
        Self { conn }
    }

    pub fn save(&self, repo: &LinkedRepo) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .expect("RepoRepository: failed to acquire database lock");
        conn.execute(
            "INSERT OR REPLACE INTO repos (id, name, path, created_at) VALUES (?1, ?2, ?3, ?4)",
            (
                &repo.id,
                &repo.name,
                repo.path.to_string_lossy().as_ref(),
                &repo.created_at,
            ),
        )?;

        // Delete existing remotes for this repo
        conn.execute("DELETE FROM repo_remotes WHERE repo_id = ?1", [&repo.id])?;

        // Insert new remotes
        for url in &repo.remotes {
            conn.execute(
                "INSERT INTO repo_remotes (repo_id, url) VALUES (?1, ?2)",
                (&repo.id, url),
            )?;
        }

        Ok(())
    }

    pub fn find_all(&self) -> Result<Vec<LinkedRepo>> {
        let conn = self
            .conn
            .lock()
            .expect("RepoRepository: failed to acquire database lock");
        let mut stmt = conn.prepare("SELECT id, name, path, created_at FROM repos")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        let mut repos = Vec::new();
        for row in rows {
            let (id, name, path_str, created_at) = row?;

            // Fetch remotes for this repo
            let mut remote_stmt =
                conn.prepare("SELECT url FROM repo_remotes WHERE repo_id = ?1")?;
            let remote_rows = remote_stmt.query_map([&id], |r| r.get::<_, String>(0))?;
            let mut remotes = Vec::new();
            for r in remote_rows {
                remotes.push(r?);
            }

            repos.push(LinkedRepo {
                id,
                name,
                path: std::path::PathBuf::from(path_str),
                remotes,
                created_at,
            });
        }
        Ok(repos)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .expect("RepoRepository: failed to acquire database lock");
        conn.execute("DELETE FROM repos WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn find_by_remote_url(&self, url_fragment: &str) -> Result<Option<LinkedRepo>> {
        let conn = self
            .conn
            .lock()
            .expect("RepoRepository: failed to acquire database lock");
        let mut stmt =
            conn.prepare("SELECT repo_id FROM repo_remotes WHERE url LIKE ?1 LIMIT 1")?;

        let mut rows = stmt.query([format!("%{}%", url_fragment)])?;
        if let Some(row) = rows.next()? {
            let repo_id: String = row.get(0)?;
            // Reuse find_all-like logic but for a single ID
            let mut repo_stmt =
                conn.prepare("SELECT id, name, path, created_at FROM repos WHERE id = ?1")?;
            let mut repo_rows = repo_stmt.query_map([&repo_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?;

            if let Some(repo_row) = repo_rows.next() {
                let (id, name, path_str, created_at) = repo_row?;
                let mut remote_stmt =
                    conn.prepare("SELECT url FROM repo_remotes WHERE repo_id = ?1")?;
                let remote_rows = remote_stmt.query_map([&id], |r| r.get::<_, String>(0))?;
                let mut remotes = Vec::new();
                for r in remote_rows {
                    remotes.push(r?);
                }

                return Ok(Some(LinkedRepo {
                    id,
                    name,
                    path: std::path::PathBuf::from(path_str),
                    remotes,
                    created_at,
                }));
            }
        }
        Ok(None)
    }
}
