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
        const SCHEMA_VERSION: i32 = 10;

        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        let existing_version: i32 =
            conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

        if existing_version == 0 {
            // Fresh database - skip migrations and go directly to current version
            Self::create_schema(&conn)?;
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

    pub fn feedback_repo(&self) -> crate::infra::db::repository::FeedbackRepository {
        crate::infra::db::repository::FeedbackRepository::new(self.connection())
    }

    pub fn feedback_link_repo(&self) -> crate::infra::db::repository::FeedbackLinkRepository {
        crate::infra::db::repository::FeedbackLinkRepository::new(self.connection())
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
                status TEXT DEFAULT 'todo',
                sub_flow TEXT,
                FOREIGN KEY(run_id) REFERENCES review_runs(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS feedback (
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
                feedback_id TEXT NOT NULL,
                author TEXT NOT NULL,
                body TEXT NOT NULL,
                parent_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY(feedback_id) REFERENCES feedback(id) ON DELETE CASCADE,
                FOREIGN KEY(parent_id) REFERENCES comments(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS feedback_links (
                id TEXT PRIMARY KEY,
                feedback_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                provider_feedback_id TEXT NOT NULL,
                provider_root_comment_id TEXT NOT NULL,
                last_synced_at TEXT NOT NULL,
                FOREIGN KEY(feedback_id) REFERENCES feedback(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_feedback_task_id ON feedback(task_id);
            CREATE INDEX IF NOT EXISTS idx_feedback_review_id ON feedback(review_id);
            CREATE INDEX IF NOT EXISTS idx_feedback_anchor ON feedback(anchor_file_path, anchor_line);
            CREATE INDEX IF NOT EXISTS idx_comments_feedback_id ON comments(feedback_id);
            CREATE INDEX IF NOT EXISTS idx_comments_feedback_created_at ON comments(feedback_id, created_at);
            "#,
        )?;
        Ok(())
    }

    /// Execute a migration for the specified version.
    ///
    /// Migration scripts are embedded into the binary at compile time to
    /// ensure reliable execution in all environments without external dependencies.
    fn run_migration(conn: &Connection, version: i32) -> Result<()> {
        // SQL is loaded from the /migrations directory in the workspace root.
        let sql = match version {
            9 => include_str!("../../../migrations/0009_update_feedback_status_constraint.sql"),
            10 => include_str!("../../../migrations/0010_rename_thread_to_feedback.sql"),
            _ => {
                return Err(anyhow::anyhow!(
                    "Unknown migration version: {}. Add the migration to run_migration() in database.rs",
                    version
                ));
            }
        };

        conn.execute_batch(sql)
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

    #[test]
    fn test_schema_migration_v9_to_v10() {
        // 1. Create a v9 database manually
        let conn = Connection::open_in_memory().unwrap();

        // Create legacy v9-style tables
        conn.execute_batch(
            "
            CREATE TABLE tasks (id TEXT PRIMARY KEY);
            CREATE TABLE review_runs (id TEXT PRIMARY KEY);
            CREATE TABLE repos (id TEXT PRIMARY KEY);
            CREATE TABLE reviews (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                summary TEXT,
                source_json TEXT,
                active_run_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE threads (
                id TEXT PRIMARY KEY,
                review_id TEXT NOT NULL,
                task_id TEXT,
                title TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'todo',
                impact TEXT NOT NULL DEFAULT 'nitpick',
                anchor_file_path TEXT,
                anchor_line INTEGER,
                anchor_side TEXT,
                anchor_hunk_ref TEXT,
                anchor_head_sha TEXT,
                author TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY(review_id) REFERENCES reviews(id)
            );
            CREATE TABLE thread_links (
                id TEXT PRIMARY KEY,
                thread_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                provider_feedback_id TEXT NOT NULL,
                provider_root_comment_id TEXT NOT NULL,
                last_synced_at TEXT NOT NULL,
                FOREIGN KEY(thread_id) REFERENCES threads(id)
            );
            CREATE TABLE comments (
                id TEXT PRIMARY KEY,
                thread_id TEXT NOT NULL,
                author TEXT NOT NULL,
                body TEXT NOT NULL,
                parent_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY(thread_id) REFERENCES threads(id)
            );
            PRAGMA user_version = 9;
        ",
        )
        .unwrap();

        // Insert legacy data
        conn.execute(
            r#"INSERT INTO reviews (id, title, created_at, updated_at) 
             VALUES ('r1', 'Legacy Review', 'now', 'now')"#,
            [],
        )
        .unwrap();
        conn.execute(
            r#"INSERT INTO threads (id, review_id, title, author, created_at, updated_at) 
             VALUES ('t1', 'r1', 'Legacy Thread', 'user', 'now', 'now')"#,
            [],
        )
        .unwrap();
        conn.execute(
            r#"INSERT INTO thread_links (id, thread_id, provider, provider_feedback_id, provider_root_comment_id, last_synced_at) 
             VALUES ('l1', 't1', 'github', 'gh1', 'c1', 'now')"#,
            [],
        )
        .unwrap();
        conn.execute(
            r#"INSERT INTO comments (id, thread_id, author, body, created_at, updated_at) 
             VALUES ('c1', 't1', 'author', 'Legacy Comment', 'now', 'now')"#,
            [],
        )
        .unwrap();

        let db = Database {
            conn: Arc::new(Mutex::new(conn)),
        };
        let conn = db.connection();

        // 2. Trigger migration to v10
        // (In a real app this happens in init() called by open())
        db.init().unwrap();

        // 3. Verify data moved to feedback and columns renamed
        let guard = conn.lock().unwrap();

        // Check feedback table
        let count: i32 = guard
            .query_row("SELECT COUNT(*) FROM feedback", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);

        let title: String = guard
            .query_row("SELECT title FROM feedback WHERE id = 't1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(title, "Legacy Thread");

        // Check feedback_links table
        let provider: String = guard
            .query_row(
                "SELECT provider FROM feedback_links WHERE feedback_id = 't1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(provider, "github");

        // Check comments table (column renamed)
        let body: String = guard
            .query_row(
                "SELECT body FROM comments WHERE feedback_id = 't1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(body, "Legacy Comment");

        // Check threads table dropped
        let threads_exists: i32 = guard
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='threads'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(threads_exists, 0);

        // Check thread_links table dropped
        let thread_links_exists: i32 = guard
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='thread_links'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(thread_links_exists, 0);
    }
}
