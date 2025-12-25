//! SQLite database setup and connection management for LaReview
//! Handles database initialization, schema creation, and connection management.

use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Database wrapper that manages SQLite connections
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Create or open the database at the default location
    pub fn open() -> Result<Self> {
        let path = Self::default_path();
        Self::open_at(path)
    }

    /// Create an in-memory database (useful for testing)
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init()?;
        Ok(db)
    }

    /// Create or open the database at a specific path
    pub fn open_at(path: PathBuf) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&path)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init()?;

        // Expose chosen path for downstream consumers (ACP worker) if not already set
        if std::env::var("LAREVIEW_DB_PATH").is_err() {
            // set_var is currently unsafe on nightly; this is limited to process-local config.
            unsafe {
                std::env::set_var("LAREVIEW_DB_PATH", path.to_string_lossy().to_string());
            }
        }
        Ok(db)
    }

    /// Get the default database path
    fn default_path() -> PathBuf {
        if let Ok(path) = std::env::var("LAREVIEW_DB_PATH") {
            return PathBuf::from(path);
        }

        #[cfg(target_os = "macos")]
        {
            if let Some(home) = home::home_dir() {
                return home
                    .join("Library")
                    .join("Application Support")
                    .join("LaReview")
                    .join("db.sqlite");
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Some(appdata) = std::env::var_os("APPDATA") {
                return PathBuf::from(appdata).join("LaReview").join("db.sqlite");
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
                return PathBuf::from(xdg).join("lareview").join("db.sqlite");
            }
            if let Some(home) = home::home_dir() {
                return home
                    .join(".local")
                    .join("share")
                    .join("lareview")
                    .join("db.sqlite");
            }
        }

        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".lareview")
            .join("db.sqlite")
    }

    /// Initialize database schema
    fn init(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        const SCHEMA_VERSION: i32 = 9;

        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        let existing_version: i32 =
            conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

        // Always ensure schema exists (using CREATE TABLE IF NOT EXISTS)
        Self::create_schema(&conn)?;

        if existing_version == 0 {
            // Fresh database - skip migrations and go directly to current version
            conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        } else if existing_version < SCHEMA_VERSION {
            // Existing database - run migrations to bring it up to date
            for version in (existing_version + 1)..=SCHEMA_VERSION {
                Self::run_migration(&conn, version)?;
            }
            conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }

        Ok(())
    }

    /// Get a reference to the connection
    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }

    pub fn task_repo(&self) -> crate::infra::db::repository::TaskRepository {
        crate::infra::db::repository::TaskRepository::new(self.connection())
    }

    pub fn thread_repo(&self) -> crate::infra::db::repository::ThreadRepository {
        crate::infra::db::repository::ThreadRepository::new(self.connection())
    }

    pub fn comment_repo(&self) -> crate::infra::db::repository::CommentRepository {
        crate::infra::db::repository::CommentRepository::new(self.connection())
    }

    pub fn review_repo(&self) -> crate::infra::db::repository::ReviewRepository {
        crate::infra::db::repository::ReviewRepository::new(self.connection())
    }

    pub fn run_repo(&self) -> crate::infra::db::repository::ReviewRunRepository {
        crate::infra::db::repository::ReviewRunRepository::new(self.connection())
    }

    pub fn repo_repo(&self) -> crate::infra::db::repository::RepoRepository {
        crate::infra::db::repository::RepoRepository::new(self.connection())
    }

    fn create_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS repos (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                path TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS repo_remotes (
                repo_id TEXT NOT NULL,
                url TEXT NOT NULL,
                PRIMARY KEY(repo_id, url),
                FOREIGN KEY(repo_id) REFERENCES repos(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS reviews (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                summary TEXT,
                source_json TEXT NOT NULL,
                active_run_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS review_runs (
                id TEXT PRIMARY KEY,
                review_id TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                input_ref TEXT NOT NULL,
                diff_text TEXT NOT NULL,
                diff_hash TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(review_id) REFERENCES reviews(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                files TEXT NOT NULL,
                stats TEXT NOT NULL,
                insight TEXT,
                diff_refs TEXT,
                diagram TEXT,
                ai_generated INTEGER DEFAULT 0,
                status TEXT DEFAULT 'PENDING',
                sub_flow TEXT,
                FOREIGN KEY(run_id) REFERENCES review_runs(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS threads (
                id TEXT PRIMARY KEY,
                review_id TEXT NOT NULL,
                task_id TEXT,
                title TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'todo' CHECK (status IN ('todo','in_progress','done','ignored','wip','reject')),
                impact TEXT NOT NULL DEFAULT 'nitpick' CHECK (impact IN ('blocking','nice_to_have','nitpick')),
                anchor_file_path TEXT,
                anchor_line INTEGER,
                anchor_side TEXT CHECK (anchor_side IN ('old','new')),
                anchor_hunk_ref TEXT,
                anchor_head_sha TEXT,
                author TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY(review_id) REFERENCES reviews(id) ON DELETE CASCADE,
                FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS comments (
                id TEXT PRIMARY KEY,
                thread_id TEXT NOT NULL,
                author TEXT NOT NULL,
                body TEXT NOT NULL,
                parent_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY(thread_id) REFERENCES threads(id) ON DELETE CASCADE,
                FOREIGN KEY(parent_id) REFERENCES comments(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS thread_links (
                id TEXT PRIMARY KEY,
                thread_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                provider_thread_id TEXT NOT NULL,
                provider_root_comment_id TEXT NOT NULL,
                last_synced_at TEXT NOT NULL,
                FOREIGN KEY(thread_id) REFERENCES threads(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_threads_task_id ON threads(task_id);
            CREATE INDEX IF NOT EXISTS idx_threads_review_id ON threads(review_id);
            CREATE INDEX IF NOT EXISTS idx_threads_anchor ON threads(anchor_file_path, anchor_line);
            CREATE INDEX IF NOT EXISTS idx_comments_thread_id ON comments(thread_id);
            CREATE INDEX IF NOT EXISTS idx_comments_thread_created_at ON comments(thread_id, created_at);
            "#,
        )?;
        Ok(())
    }

    /// Run a migration by loading and executing the corresponding SQL file
    fn run_migration(conn: &Connection, version: i32) -> Result<()> {
        // Migration files are expected to be in migrations/ folder at the project root
        // with naming pattern: {version:04}_description.sql

        // Find the migration file for this version
        let migrations_dir = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
            .and_then(|mut p| {
                // Try relative to executable first (for installed builds)
                p.push("migrations");
                if p.exists() {
                    Some(p)
                } else {
                    // Fall back to workspace root for development
                    std::env::current_dir()
                        .ok()
                        .map(|cwd| cwd.join("migrations"))
                }
            })
            .unwrap_or_else(|| PathBuf::from("migrations"));

        // Find migration file matching this version
        let version_prefix = format!("{:04}_", version);

        let migration_file = std::fs::read_dir(&migrations_dir)
            .map_err(|e| anyhow::anyhow!("Failed to read migrations directory: {}", e))?
            .filter_map(|entry| entry.ok())
            .find(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(&version_prefix)
                    && entry.file_name().to_string_lossy().ends_with(".sql")
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Migration file not found for version {} in {}",
                    version,
                    migrations_dir.display()
                )
            })?;

        // Read and execute the migration SQL
        let sql = std::fs::read_to_string(migration_file.path())
            .map_err(|e| anyhow::anyhow!("Failed to read migration file: {}", e))?;

        conn.execute_batch(&sql)
            .map_err(|e| anyhow::anyhow!("Failed to execute migration {}: {}", version, e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_default_path() {
        let path = Database::default_path();
        assert!(path.to_string_lossy().contains("db.sqlite"));
    }

    #[test]
    fn test_database_open_in_memory() {
        let db = Database::open_in_memory().unwrap();
        let conn = db.connection();
        let guard = conn.lock().unwrap();
        let res: i32 = guard.query_row("SELECT 1", [], |row| row.get(0)).unwrap();
        assert_eq!(res, 1);
    }
}
