use crate::commands::{LinkedRepoState, PendingReviewState, ReviewRunState, ReviewState};
use crate::domain::{Comment, Feedback, Review, ReviewRun, ReviewTask};
use anyhow::Result;
use rusqlite::{Connection, params};
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
                created_at TEXT NOT NULL,
                allow_snapshot_access INTEGER DEFAULT 0
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
                status TEXT NOT NULL DEFAULT 'todo',
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
                status TEXT NOT NULL DEFAULT 'completed',
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
                rule_id TEXT,
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

            CREATE TABLE IF NOT EXISTS review_rules (
                id TEXT PRIMARY KEY,
                rule_type TEXT NOT NULL DEFAULT 'guideline',
                scope TEXT NOT NULL,
                repo_id TEXT,
                glob TEXT,
                category TEXT,
                text TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY(repo_id) REFERENCES repos(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS issue_checks (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                rule_id TEXT,
                category TEXT NOT NULL,
                display_name TEXT NOT NULL,
                status TEXT NOT NULL,
                confidence TEXT NOT NULL,
                summary TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY(run_id) REFERENCES review_runs(id) ON DELETE CASCADE,
                FOREIGN KEY(rule_id) REFERENCES review_rules(id) ON DELETE SET NULL
            );

            CREATE TABLE IF NOT EXISTS issue_findings (
                id TEXT PRIMARY KEY,
                check_id TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                evidence TEXT NOT NULL,
                file_path TEXT,
                line_number INTEGER,
                impact TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(check_id) REFERENCES issue_checks(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_feedback_task_id ON feedback(task_id);
            CREATE INDEX IF NOT EXISTS idx_feedback_review_id ON feedback(review_id);
            CREATE INDEX IF NOT EXISTS idx_feedback_anchor ON feedback(anchor_file_path, anchor_line);
            CREATE INDEX IF NOT EXISTS idx_comments_feedback_id ON comments(feedback_id);
            CREATE INDEX IF NOT EXISTS idx_comments_feedback_created_at ON comments(feedback_id, created_at);
            CREATE INDEX IF NOT EXISTS idx_review_rules_repo_id ON review_rules(repo_id);
            CREATE INDEX IF NOT EXISTS idx_review_rules_scope ON review_rules(scope);
            CREATE INDEX IF NOT EXISTS idx_issue_checks_run_id ON issue_checks(run_id);
            CREATE INDEX IF NOT EXISTS idx_issue_checks_category ON issue_checks(category);
            CREATE INDEX IF NOT EXISTS idx_issue_findings_check_id ON issue_findings(check_id);

            "#,
        )?;

        // Migration: Add status to reviews if it doesn't exist
        let has_status = conn
            .prepare("SELECT 1 FROM pragma_table_info('reviews') WHERE name = 'status'")?
            .exists([])?;

        if !has_status {
            conn.execute(
                "ALTER TABLE reviews ADD COLUMN status TEXT DEFAULT 'todo' NOT NULL",
                [],
            )?;
        }

        let has_rule_id = conn
            .prepare("SELECT 1 FROM pragma_table_info('feedback') WHERE name = 'rule_id'")?
            .exists([])?;

        if !has_rule_id {
            conn.execute("ALTER TABLE feedback ADD COLUMN rule_id TEXT", [])?;
        }

        Self::migrate_legacy_custom_rules(conn)?;

        // Migration: Add status to review_runs if it doesn't exist
        let has_run_status = conn
            .prepare("SELECT 1 FROM pragma_table_info('review_runs') WHERE name = 'status'")?
            .exists([])?;

        if !has_run_status {
            conn.execute(
                "ALTER TABLE review_runs ADD COLUMN status TEXT DEFAULT 'completed' NOT NULL",
                [],
            )?;
        }

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_review_runs_status ON review_runs(status)",
            [],
        )?;

        // Migration: Add allow_snapshot_access to repos if it doesn't exist
        let has_snapshot_access = conn
            .prepare(
                "SELECT 1 FROM pragma_table_info('repos') WHERE name = 'allow_snapshot_access'",
            )?
            .exists([])?;

        if !has_snapshot_access {
            conn.execute(
                "ALTER TABLE repos ADD COLUMN allow_snapshot_access INTEGER DEFAULT 0",
                [],
            )?;
        }

        // Migration: Add rule_type to review_rules if it doesn't exist
        let has_rule_type = conn
            .prepare("SELECT 1 FROM pragma_table_info('review_rules') WHERE name = 'rule_type'")?
            .exists([])?;

        if !has_rule_type {
            conn.execute(
                "ALTER TABLE review_rules ADD COLUMN rule_type TEXT DEFAULT 'guideline' NOT NULL",
                [],
            )?;
        }

        // Migration: Add category to review_rules if it doesn't exist
        let has_category = conn
            .prepare("SELECT 1 FROM pragma_table_info('review_rules') WHERE name = 'category'")?
            .exists([])?;

        if !has_category {
            conn.execute("ALTER TABLE review_rules ADD COLUMN category TEXT", [])?;
        }

        // Migration: Add finding_id to feedback if it doesn't exist
        let has_finding_id = conn
            .prepare("SELECT 1 FROM pragma_table_info('feedback') WHERE name = 'finding_id'")?
            .exists([])?;

        if !has_finding_id {
            conn.execute("ALTER TABLE feedback ADD COLUMN finding_id TEXT", [])?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_feedback_finding_id ON feedback(finding_id)",
                [],
            )?;
        }

        Ok(())
    }

    fn migrate_legacy_custom_rules(conn: &Connection) -> Result<()> {
        let has_custom_rules = Self::table_exists(conn, "custom_rules")?;
        let has_custom_rules_repos = Self::table_exists(conn, "custom_rules_repos")?;
        let has_feedback_rules = Self::table_exists(conn, "feedback_rules")?;
        let feedback_fk_custom = Self::feedback_fk_targets(conn)?
            .iter()
            .any(|target| target == "custom_rules");

        if !has_custom_rules
            && !has_custom_rules_repos
            && !has_feedback_rules
            && !feedback_fk_custom
        {
            return Ok(());
        }

        conn.execute_batch("PRAGMA foreign_keys = OFF;")?;
        conn.execute_batch("BEGIN;")?;

        let result = (|| {
            if feedback_fk_custom {
                Self::rebuild_feedback_without_custom_rules_fk(conn)?;
            }

            conn.execute("DROP TABLE IF EXISTS feedback_rules", [])?;
            conn.execute("DROP TABLE IF EXISTS custom_rules_repos", [])?;
            conn.execute("DROP TABLE IF EXISTS custom_rules", [])?;

            Ok(())
        })();

        match result {
            Ok(()) => {
                conn.execute_batch("COMMIT;")?;
            }
            Err(err) => {
                let _ = conn.execute_batch("ROLLBACK;");
                conn.execute_batch("PRAGMA foreign_keys = ON;")?;
                return Err(err);
            }
        }

        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        Ok(())
    }

    fn rebuild_feedback_without_custom_rules_fk(conn: &Connection) -> Result<()> {
        if !Self::table_exists(conn, "feedback")? {
            return Ok(());
        }

        let has_rule_name = Self::column_exists(conn, "feedback", "rule_name")?;
        let has_anchor_diff_line_idx =
            Self::column_exists(conn, "feedback", "anchor_diff_line_idx")?;
        let has_anchor_diff_hash = Self::column_exists(conn, "feedback", "anchor_diff_hash")?;

        let mut extra_defs = Vec::new();
        if has_rule_name {
            extra_defs.push("rule_name TEXT");
        }
        if has_anchor_diff_line_idx {
            extra_defs.push("anchor_diff_line_idx INTEGER");
        }
        if has_anchor_diff_hash {
            extra_defs.push("anchor_diff_hash TEXT");
        }

        let extra_defs = if extra_defs.is_empty() {
            String::new()
        } else {
            format!(
                ",\n                {}",
                extra_defs.join(",\n                ")
            )
        };

        let create_sql = format!(
            r#"
            CREATE TABLE feedback_new (
                id TEXT PRIMARY KEY,
                review_id TEXT NOT NULL,
                task_id TEXT,
                rule_id TEXT,
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
                updated_at TEXT NOT NULL{extra_defs},
                FOREIGN KEY(review_id) REFERENCES reviews(id) ON DELETE CASCADE,
                FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE
            );
            "#
        );
        conn.execute_batch(&create_sql)?;

        let mut columns = vec![
            "id",
            "review_id",
            "task_id",
            "rule_id",
            "title",
            "status",
            "impact",
            "anchor_file_path",
            "anchor_line",
            "anchor_side",
            "anchor_hunk_ref",
            "anchor_head_sha",
            "author",
            "created_at",
            "updated_at",
        ];
        if has_rule_name {
            columns.push("rule_name");
        }
        if has_anchor_diff_line_idx {
            columns.push("anchor_diff_line_idx");
        }
        if has_anchor_diff_hash {
            columns.push("anchor_diff_hash");
        }

        let column_list = columns.join(", ");
        let insert_sql =
            format!("INSERT INTO feedback_new ({column_list}) SELECT {column_list} FROM feedback");
        conn.execute(&insert_sql, [])?;

        conn.execute("DROP TABLE feedback", [])?;
        conn.execute("ALTER TABLE feedback_new RENAME TO feedback", [])?;

        conn.execute_batch(
            r#"
            CREATE INDEX IF NOT EXISTS idx_feedback_task_id ON feedback(task_id);
            CREATE INDEX IF NOT EXISTS idx_feedback_review_id ON feedback(review_id);
            CREATE INDEX IF NOT EXISTS idx_feedback_anchor ON feedback(anchor_file_path, anchor_line);
            "#,
        )?;

        Ok(())
    }

    fn table_exists(conn: &Connection, name: &str) -> Result<bool> {
        let mut stmt =
            conn.prepare("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1")?;
        Ok(stmt.exists([name])?)
    }

    fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
        let sql = format!("SELECT 1 FROM pragma_table_info('{table}') WHERE name = '{column}'");
        let mut stmt = conn.prepare(&sql)?;
        Ok(stmt.exists([])?)
    }

    fn feedback_fk_targets(conn: &Connection) -> Result<Vec<String>> {
        let mut stmt = conn.prepare("PRAGMA foreign_key_list('feedback')")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(2))?;
        let mut targets = Vec::new();
        for row in rows {
            targets.push(row?);
        }
        Ok(targets)
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

    pub fn rule_repo(&self) -> crate::infra::db::repository::ReviewRuleRepository {
        crate::infra::db::repository::ReviewRuleRepository::new(self.connection())
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

    pub fn issue_check_repo(&self) -> crate::infra::db::repository::IssueCheckRepository {
        crate::infra::db::repository::IssueCheckRepository::new(self.connection())
    }

    pub fn get_pending_reviews(&self) -> Result<Vec<PendingReviewState>, rusqlite::Error> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        let mut stmt = conn.prepare(
            "SELECT r.id, r.active_run_id, r.source_json, r.created_at, r.updated_at
             FROM reviews r
             JOIN review_runs rr ON r.active_run_id = rr.id
             WHERE rr.status IN ('running', 'queued')
             ORDER BY r.updated_at DESC
             LIMIT 10",
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
            "SELECT r.id, r.title, r.summary, rr.agent_id, COUNT(t.id) as task_count, r.created_at, r.source_json, r.status, rr.status
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

            let status_str: String = row.get(7)?;
            let active_run_status: Option<String> = row.get(8)?;

            Ok(ReviewState {
                id: row.get(0)?,
                title: row.get(1)?,
                summary: row.get(2)?,
                agent_id: row.get(3)?,
                task_count: row.get::<_, i32>(4)? as usize,
                created_at: row.get(5)?,
                source,
                status: status_str,
                active_run_status,
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
            "SELECT rr.id, rr.review_id, rr.agent_id, rr.input_ref, rr.diff_text, rr.status, rr.created_at, COUNT(t.id) as task_count
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
                status: row.get(5)?,
                created_at: row.get(6)?,
                task_count: row.get::<_, i32>(7)? as usize,
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
            conn.prepare("SELECT id, name, path, created_at, allow_snapshot_access FROM repos ORDER BY created_at DESC")?;

        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let path: String = row.get(2)?;
            let linked_at: String = row.get(3)?;
            let allow_snapshot_access: bool = row.get::<_, Option<bool>>(4)?.unwrap_or(false);

            Ok((id, name, path, linked_at, allow_snapshot_access))
        })?;

        let mut repos = Vec::new();
        for row in rows {
            let (id, name, path, linked_at, allow_snapshot_access) = row?;

            // Fetch remotes
            let mut remote_stmt =
                conn.prepare("SELECT url FROM repo_remotes WHERE repo_id = ?1")?;
            let remote_rows = remote_stmt.query_map([&id], |r| r.get::<_, String>(0))?;
            let mut remotes = Vec::new();
            for r in remote_rows {
                remotes.push(r?);
            }

            repos.push(LinkedRepoState {
                id,
                name,
                path,
                review_count: 0,
                linked_at,
                remotes,
                allow_snapshot_access,
            });
        }
        Ok(repos)
    }

    pub fn save_review(&self, review: &Review) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        let source_json = serde_json::to_string(&review.source).map_err(|e| {
            rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e.to_string()))
        })?;
        conn.execute(
            "INSERT INTO reviews (id, title, summary, source_json, active_run_id, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO NOTHING",
            params![
                &review.id,
                &review.title,
                &review.summary,
                &source_json,
                &review.active_run_id,
                &review.status.to_string(),
                &review.created_at,
                &review.updated_at,
            ],
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
            "INSERT INTO review_runs (id, review_id, agent_id, input_ref, diff_text, diff_hash, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO NOTHING",
            params![
                &run.id,
                &run.review_id,
                &run.agent_id,
                &run.input_ref,
                &run.diff_text,
                &run.diff_hash,
                &run.status.to_string(),
                &run.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn save_feedback(&self, feedback: &Feedback, _id: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        let anchor = feedback.anchor.as_ref();
        conn.execute(
            "INSERT INTO feedback (id, review_id, task_id, rule_id, title, status, impact, anchor_file_path, anchor_line, anchor_side, anchor_hunk_ref, anchor_head_sha, author, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT(id) DO NOTHING",
            params![
                &feedback.id,
                &feedback.review_id,
                &feedback.task_id.as_ref(),
                &feedback.rule_id.as_ref(),
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
            ],
        )?;
        Ok(())
    }

    pub fn save_comment(&self, comment: &Comment) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        conn.execute(
            "INSERT INTO comments (id, feedback_id, author, body, parent_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO NOTHING",
            params![
                &comment.id,
                &comment.feedback_id,
                &comment.author,
                &comment.body,
                &comment.parent_id,
                &comment.created_at,
                &comment.updated_at,
            ],
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
        repo.update_status(&feedback_id_str, status)
            .map_err(|e: anyhow::Error| {
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

    pub fn mark_stale_runs_failed(&self) -> Result<usize, rusqlite::Error> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        conn.execute(
            "UPDATE review_runs SET status = 'failed' WHERE status IN ('running', 'queued')",
            [],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Review, ReviewRun, ReviewRunStatus, ReviewSource, ReviewStatus};

    #[test]
    fn test_get_all_reviews_includes_active_run_status() -> anyhow::Result<()> {
        let db = Database::open_in_memory()?;

        let review = Review {
            id: "rev-1".to_string(),
            title: "Test Review".to_string(),
            summary: None,
            source: ReviewSource::DiffPaste {
                diff_hash: "h".into(),
            },
            active_run_id: Some("run-1".into()),
            status: ReviewStatus::Todo,
            created_at: "now".to_string(),
            updated_at: "now".to_string(),
        };
        db.save_review(&review)?;

        let run = ReviewRun {
            id: "run-1".into(),
            review_id: review.id.clone(),
            agent_id: "agent".into(),
            input_ref: "input".into(),
            diff_text: "diff".into(),
            diff_hash: "h".into(),
            status: ReviewRunStatus::Running,
            created_at: "now".into(),
        };
        db.save_run(&run)?;

        let reviews = db.get_all_reviews()?;
        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0].active_run_status.as_deref(), Some("running"));

        Ok(())
    }

    #[test]
    fn test_get_pending_reviews_filters_by_run_status() -> anyhow::Result<()> {
        let db = Database::open_in_memory()?;

        let running_review = Review {
            id: "rev-running".to_string(),
            title: "Running Review".to_string(),
            summary: None,
            source: ReviewSource::DiffPaste {
                diff_hash: "h".into(),
            },
            active_run_id: Some("run-1".into()),
            status: ReviewStatus::Todo,
            created_at: "now".to_string(),
            updated_at: "now".to_string(),
        };
        db.save_review(&running_review)?;

        let completed_review = Review {
            id: "rev-completed".to_string(),
            title: "Completed Review".to_string(),
            summary: None,
            source: ReviewSource::DiffPaste {
                diff_hash: "h2".into(),
            },
            active_run_id: Some("run-2".into()),
            status: ReviewStatus::Todo,
            created_at: "now".to_string(),
            updated_at: "now".to_string(),
        };
        db.save_review(&completed_review)?;

        db.save_run(&ReviewRun {
            id: "run-1".into(),
            review_id: running_review.id.clone(),
            agent_id: "agent".into(),
            input_ref: "input".into(),
            diff_text: "diff".into(),
            diff_hash: "h".into(),
            status: ReviewRunStatus::Running,
            created_at: "now".into(),
        })?;

        db.save_run(&ReviewRun {
            id: "run-2".into(),
            review_id: completed_review.id.clone(),
            agent_id: "agent".into(),
            input_ref: "input".into(),
            diff_text: "diff".into(),
            diff_hash: "h2".into(),
            status: ReviewRunStatus::Completed,
            created_at: "now".into(),
        })?;

        let pending = db.get_pending_reviews()?;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, running_review.id);

        Ok(())
    }

    #[test]
    fn test_mark_stale_runs_failed() -> anyhow::Result<()> {
        let db = Database::open_in_memory()?;

        let review = Review {
            id: "rev-stale".to_string(),
            title: "Stale Review".to_string(),
            summary: None,
            source: ReviewSource::DiffPaste {
                diff_hash: "h".into(),
            },
            active_run_id: Some("run-stale".into()),
            status: ReviewStatus::Todo,
            created_at: "now".to_string(),
            updated_at: "now".to_string(),
        };
        db.save_review(&review)?;

        db.save_run(&ReviewRun {
            id: "run-stale".into(),
            review_id: review.id.clone(),
            agent_id: "agent".into(),
            input_ref: "input".into(),
            diff_text: "diff".into(),
            diff_hash: "h".into(),
            status: ReviewRunStatus::Running,
            created_at: "now".into(),
        })?;

        let updated = db.mark_stale_runs_failed()?;
        assert_eq!(updated, 1);

        let run = db
            .run_repo()
            .find_by_id(&"run-stale".into())?
            .expect("run exists");
        assert_eq!(run.status, ReviewRunStatus::Failed);

        Ok(())
    }
}
