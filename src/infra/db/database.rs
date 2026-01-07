use crate::commands::{LinkedRepoState, PendingReviewState, ReviewRunState, ReviewState};
use crate::domain::{Comment, Feedback, Review, ReviewRun, ReviewTask};
use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open() -> Result<Self> {
        let path = Self::default_path();
        Self::open_at(path)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init()?;
        Ok(db)
    }

    pub fn open_at(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&path)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init()?;

        if std::env::var("LAREVIEW_DB_PATH").is_err() {
            unsafe {
                std::env::set_var("LAREVIEW_DB_PATH", path.to_string_lossy().to_string());
            }
        }
        Ok(db)
    }

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

    fn init(&self) -> Result<()> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        Self::create_schema(&conn)?;
        Ok(())
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

            CREATE INDEX IF NOT EXISTS idx_feedback_task_id ON feedback(task_id);
            CREATE INDEX IF NOT EXISTS idx_feedback_review_id ON feedback(review_id);
            CREATE INDEX IF NOT EXISTS idx_feedback_anchor ON feedback(anchor_file_path, anchor_line);
            CREATE INDEX IF NOT EXISTS idx_comments_feedback_id ON comments(feedback_id);
            CREATE INDEX IF NOT EXISTS idx_comments_feedback_created_at ON comments(feedback_id, created_at);
            "#,
        )?;
        Ok(())
    }

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

    pub fn get_pending_reviews(&self) -> Result<Vec<PendingReviewState>, rusqlite::Error> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        let mut stmt = conn.prepare(
            "SELECT id, active_run_id, source_json, created_at, updated_at FROM reviews WHERE active_run_id IS NOT NULL ORDER BY updated_at DESC LIMIT 10"
        )?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let source_json: String = row.get(2)?;
            let _source: crate::domain::ReviewSource = serde_json::from_str(&source_json)
                .unwrap_or_else(|_| crate::domain::ReviewSource::DiffPaste {
                    diff_hash: String::new(),
                });

            Ok(PendingReviewState {
                id: id.clone(),
                diff: String::new(),
                repo_root: None,
                agent: None,
                source: format!("review-{}", id),
                created_at: row.get(3)?,
                review_source: None,
            })
        })?;
        let mut reviews = Vec::new();
        for row in rows {
            reviews.push(row?);
        }
        Ok(reviews)
    }

    pub fn get_all_tasks(&self) -> Result<Vec<ReviewTask>, rusqlite::Error> {
        let repo = self.task_repo();
        repo.list().map_err(|e| {
            rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e.to_string()))
        })
    }

    pub fn get_tasks_by_run(&self, run_id: &str) -> Result<Vec<ReviewTask>, rusqlite::Error> {
        let repo = self.task_repo();
        let run_id_str = run_id.to_string();
        repo.find_by_run(&run_id_str).map_err(|e| {
            rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e.to_string()))
        })
    }

    pub fn get_all_reviews(&self) -> Result<Vec<ReviewState>, rusqlite::Error> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        let mut stmt = conn.prepare(
            "SELECT r.id, r.title, r.summary, rr.agent_id, COUNT(t.id) as task_count, r.created_at, r.source_json
             FROM reviews r
             LEFT JOIN review_runs rr ON r.active_run_id = rr.id
             LEFT JOIN tasks t ON t.run_id = rr.id
             GROUP BY r.id
             ORDER BY r.updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let source_json: String = row.get(6)?;
            let source: crate::domain::ReviewSource = serde_json::from_str(&source_json)
                .unwrap_or_else(|_| crate::domain::ReviewSource::DiffPaste {
                    diff_hash: String::new(),
                });

            Ok(ReviewState {
                id: row.get(0)?,
                title: row.get(1)?,
                summary: row.get(2)?,
                agent_id: row.get(3)?,
                task_count: row.get::<_, i32>(4)? as usize,
                created_at: row.get(5)?,
                source,
            })
        })?;
        let mut reviews = Vec::new();
        for row in rows {
            reviews.push(row?);
        }
        Ok(reviews)
    }

    pub fn get_review_runs(&self, review_id: &str) -> Result<Vec<ReviewRunState>, rusqlite::Error> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        let mut stmt = conn.prepare(
            "SELECT rr.id, rr.review_id, rr.agent_id, rr.input_ref, rr.diff_text, rr.created_at, COUNT(t.id) as task_count
             FROM review_runs rr
             LEFT JOIN tasks t ON t.run_id = rr.id
             WHERE rr.review_id = ?1
             GROUP BY rr.id
             ORDER BY rr.created_at DESC",
        )?;
        let rows = stmt.query_map([review_id], |row| {
            Ok(ReviewRunState {
                id: row.get(0)?,
                review_id: row.get(1)?,
                agent_id: row.get(2)?,
                input_ref: row.get(3)?,
                diff_text: row.get(4)?,
                created_at: row.get(5)?,
                task_count: row.get::<_, i32>(6)? as usize,
            })
        })?;
        let mut runs = Vec::new();
        for row in rows {
            runs.push(row?);
        }
        Ok(runs)
    }

    pub fn get_review_run_by_id(&self, run_id: &str) -> Result<Option<ReviewRun>, rusqlite::Error> {
        let repo = self.run_repo();
        let run_id_str = run_id.to_string();
        repo.find_by_id(&run_id_str).map_err(|e: anyhow::Error| {
            rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e.to_string()))
        })
    }

    pub fn get_linked_repos(&self) -> Result<Vec<LinkedRepoState>, rusqlite::Error> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        let mut stmt =
            conn.prepare("SELECT id, name, path, created_at FROM repos ORDER BY created_at DESC")?;
        let rows = stmt.query_map([], |row| {
            Ok(LinkedRepoState {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                review_count: 0,
                linked_at: row.get(3)?,
            })
        })?;
        let mut repos = Vec::new();
        for row in rows {
            repos.push(row?);
        }
        Ok(repos)
    }

    pub fn save_review(&self, review: &Review) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        let source_json = serde_json::to_string(&review.source).map_err(|e| {
            rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e.to_string()))
        })?;
        conn.execute(
            r#"
            INSERT INTO reviews (id, title, summary, source_json, active_run_id, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(id) DO NOTHING
            "#,
            (
                &review.id,
                &review.title,
                &review.summary,
                &source_json,
                &review.active_run_id,
                &review.created_at,
                &review.updated_at,
            ),
        )?;
        Ok(())
    }

    pub fn get_review(&self, review_id: &str) -> Result<Option<Review>, rusqlite::Error> {
        let review_id_str = review_id.to_string();
        self.review_repo().find_by_id(&review_id_str).map_err(|e| {
            rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e.to_string()))
        })
    }

    pub fn update_task_status(
        &self,
        task_id: &str,
        status: crate::domain::ReviewStatus,
    ) -> Result<(), rusqlite::Error> {
        let repo = self.task_repo();
        let task_id_str = task_id.to_string();
        repo.update_status(&task_id_str, status).map_err(|e| {
            rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e.to_string()))
        })
    }

    pub fn save_run(&self, run: &ReviewRun) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        conn.execute(
            r#"
            INSERT INTO review_runs (id, review_id, agent_id, input_ref, diff_text, diff_hash, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(id) DO NOTHING
            "#,
            (
                &run.id,
                &run.review_id,
                &run.agent_id,
                &run.input_ref,
                &run.diff_text,
                &run.diff_hash,
                &run.created_at,
            ),
        )?;
        Ok(())
    }

    pub fn save_feedback(&self, feedback: &Feedback, _id: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        let anchor = feedback.anchor.as_ref();
        conn.execute(
            r#"
            INSERT INTO feedback (id, review_id, task_id, title, status, impact, anchor_file_path, anchor_line, anchor_side, anchor_hunk_ref, anchor_head_sha, author, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            ON CONFLICT(id) DO NOTHING
            "#,
            (
                &feedback.id,
                &feedback.review_id,
                &feedback.task_id.as_ref(),
                &feedback.title,
                &feedback.status.to_string(),
                &feedback.impact.to_string(),
                anchor.and_then(|a| a.file_path.as_deref()),
                anchor.and_then(|a| a.line_number.map(|l| l as i32)),
                anchor.and_then(|a| a.side.map(|s| s.to_string())),
                Option::<String>::None,
                anchor.and_then(|a| a.head_sha.as_deref()),
                &feedback.author,
                &feedback.created_at,
                &feedback.updated_at,
            ),
        )?;
        Ok(())
    }

    pub fn save_comment(&self, comment: &Comment) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        conn.execute(
            r#"
            INSERT INTO comments (id, feedback_id, author, body, parent_id, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(id) DO NOTHING
            "#,
            (
                &comment.id,
                &comment.feedback_id,
                &comment.author,
                &comment.body,
                &comment.parent_id,
                &comment.created_at,
                &comment.updated_at,
            ),
        )?;
        Ok(())
    }

    pub fn get_comments_for_feedback(
        &self,
        feedback_id: &str,
    ) -> Result<Vec<Comment>, rusqlite::Error> {
        let feedback_id_str = feedback_id.to_string();
        self.comment_repo()
            .list_for_feedback(&feedback_id_str)
            .map_err(|e| {
                rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e.to_string()))
            })
    }

    pub fn get_feedback_by_review(
        &self,
        review_id: &str,
    ) -> Result<Vec<Feedback>, rusqlite::Error> {
        let review_id_str = review_id.to_string();
        self.feedback_repo()
            .find_by_review(&review_id_str)
            .map_err(|e| {
                rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e.to_string()))
            })
    }

    pub fn update_feedback_status(
        &self,
        feedback_id: &str,
        status: crate::domain::ReviewStatus,
    ) -> Result<(), rusqlite::Error> {
        let repo = self.feedback_repo();
        let feedback_id_str = feedback_id.to_string();
        repo.update_status(&feedback_id_str, status).map_err(|e| {
            rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e.to_string()))
        })?;
        Ok(())
    }

    pub fn update_feedback_impact(
        &self,
        feedback_id: &str,
        impact: crate::domain::FeedbackImpact,
    ) -> Result<(), rusqlite::Error> {
        let repo = self.feedback_repo();
        let feedback_id_str = feedback_id.to_string();
        repo.update_impact(&feedback_id_str, impact).map_err(|e| {
            rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e.to_string()))
        })?;
        Ok(())
    }

    pub fn delete_feedback(&self, feedback_id: &str) -> Result<(), rusqlite::Error> {
        let repo = self.feedback_repo();
        let feedback_id_str = feedback_id.to_string();
        repo.delete(&feedback_id_str).map_err(|e| {
            rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e.to_string()))
        })?;
        Ok(())
    }
}
