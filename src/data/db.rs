//! SQLite database setup and connection management for LaReview
//! Handles database initialization, schema creation, and connection management.

use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Database wrapper that manages SQLite connections
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    path: PathBuf,
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
            path: path.clone(),
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
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS pull_requests (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT,
                repo TEXT NOT NULL,
                author TEXT NOT NULL,
                branch TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                pull_request_id TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                files TEXT NOT NULL,
                stats TEXT NOT NULL,
                insight TEXT,
                diffs TEXT,
                diagram TEXT,
                ai_generated INTEGER DEFAULT 0,
                status TEXT DEFAULT 'PENDING',
                sub_flow TEXT,
                FOREIGN KEY(pull_request_id) REFERENCES pull_requests(id)
            );

            CREATE TABLE IF NOT EXISTS notes (
                task_id TEXT,
                file_path TEXT,
                line_number INTEGER,
                body TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (task_id, file_path, line_number)
            );

            CREATE TABLE IF NOT EXISTS diffs (
                pull_request_id TEXT NOT NULL,
                file_path TEXT NOT NULL,
                hunks TEXT NOT NULL,
                PRIMARY KEY (pull_request_id, file_path),
                FOREIGN KEY(pull_request_id) REFERENCES pull_requests(id)
            );

            CREATE TABLE IF NOT EXISTS plans (
                pr_id TEXT NOT NULL,
                plan_data TEXT NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (pr_id),
                FOREIGN KEY(pr_id) REFERENCES pull_requests(id)
            );
            "#,
        )?;

        // Try to add the sub_flow column - ignore error if it already exists
        let _ = conn.execute("ALTER TABLE tasks ADD COLUMN sub_flow TEXT", []);

        Ok(())
    }

    /// Get a reference to the connection
    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }

    /// Path backing this database
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }
}
