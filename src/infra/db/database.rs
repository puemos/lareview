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
        const SCHEMA_VERSION: i32 = 3;

        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        let existing_version: i32 =
            conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

        if existing_version != SCHEMA_VERSION {
            // Unreleased app: reset schema to the current Review-centric model.
            conn.execute_batch(
                r#"
                DROP TABLE IF EXISTS diffs;
                DROP TABLE IF EXISTS plans;
                DROP TABLE IF EXISTS tasks;
                DROP TABLE IF EXISTS notes;
                DROP TABLE IF EXISTS review_runs;
                DROP TABLE IF EXISTS reviews;
                DROP TABLE IF EXISTS pull_requests;

                CREATE TABLE reviews (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    summary TEXT,
                    source_json TEXT NOT NULL,
                    active_run_id TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );

                CREATE TABLE review_runs (
                    id TEXT PRIMARY KEY,
                    review_id TEXT NOT NULL,
                    agent_id TEXT NOT NULL,
                    input_ref TEXT NOT NULL,
                    diff_text TEXT NOT NULL,
                    diff_hash TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    FOREIGN KEY(review_id) REFERENCES reviews(id) ON DELETE CASCADE
                );

                CREATE TABLE tasks (
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

                CREATE TABLE notes (
                    task_id TEXT,
                    file_path TEXT,
                    line_number INTEGER,
                    body TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    PRIMARY KEY (task_id, file_path, line_number)
                );
                "#,
            )?;

            conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }

        Ok(())
    }

    /// Get a reference to the connection
    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }
}
