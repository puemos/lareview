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

        let cwd = std::env::current_dir().unwrap_or_default();
        cwd.join(".lareview").join("db.sqlite")
    }

    /// Initialize database schema
    fn init(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        const SCHEMA_VERSION: i32 = 8;

        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        let existing_version: i32 =
            conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

        if existing_version != SCHEMA_VERSION {
            // Breaking change: reset schema and bump version.
            Self::reset_schema(&conn)?;
            conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }

        Ok(())
    }

    /// Get a reference to the connection
    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }

    fn reset_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            DROP TABLE IF EXISTS repos;
            DROP TABLE IF EXISTS repo_remotes;
            DROP TABLE IF EXISTS diffs;
            DROP TABLE IF EXISTS plans;
            DROP TABLE IF EXISTS thread_links;
            DROP TABLE IF EXISTS comments;
            DROP TABLE IF EXISTS threads;
            DROP TABLE IF EXISTS tasks;
            DROP TABLE IF EXISTS review_runs;
            DROP TABLE IF EXISTS reviews;
            DROP TABLE IF EXISTS pull_requests;
            "#,
        )?;
        Self::create_schema(conn)
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
                status TEXT NOT NULL DEFAULT 'todo' CHECK (status IN ('todo','wip','done','reject')),
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
}
