use super::DbConn;
use crate::domain::{LearnedPattern, LearnedPatternInput, LearningStatus};
use anyhow::Result;
use chrono::Utc;
use rusqlite::OptionalExtension;
use uuid::Uuid;

/// Repository for learned patterns (AI-generated negative examples from rejected feedback)
pub struct LearnedPatternRepository {
    conn: DbConn,
}

impl LearnedPatternRepository {
    pub fn new(conn: DbConn) -> Self {
        Self { conn }
    }

    /// List all enabled patterns for injection into review prompts
    pub fn list_enabled(&self) -> Result<Vec<LearnedPattern>> {
        let conn = self
            .conn
            .lock()
            .expect("LearnedPatternRepository: failed to acquire database lock");

        let mut stmt = conn.prepare(
            r#"
            SELECT id, pattern_text, category, file_extension, source_count,
                   is_edited, enabled, created_at, updated_at
            FROM learned_patterns
            WHERE enabled = 1
            ORDER BY source_count DESC, created_at DESC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(LearnedPattern {
                id: row.get(0)?,
                pattern_text: row.get(1)?,
                category: row.get(2)?,
                file_extension: row.get(3)?,
                source_count: row.get(4)?,
                is_edited: row.get::<_, i32>(5)? != 0,
                enabled: row.get::<_, i32>(6)? != 0,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// List all patterns (including disabled) for management UI
    pub fn list_all(&self) -> Result<Vec<LearnedPattern>> {
        let conn = self
            .conn
            .lock()
            .expect("LearnedPatternRepository: failed to acquire database lock");

        let mut stmt = conn.prepare(
            r#"
            SELECT id, pattern_text, category, file_extension, source_count,
                   is_edited, enabled, created_at, updated_at
            FROM learned_patterns
            ORDER BY enabled DESC, source_count DESC, created_at DESC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(LearnedPattern {
                id: row.get(0)?,
                pattern_text: row.get(1)?,
                category: row.get(2)?,
                file_extension: row.get(3)?,
                source_count: row.get(4)?,
                is_edited: row.get::<_, i32>(5)? != 0,
                enabled: row.get::<_, i32>(6)? != 0,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Get a pattern by ID
    pub fn find_by_id(&self, id: &str) -> Result<Option<LearnedPattern>> {
        let conn = self
            .conn
            .lock()
            .expect("LearnedPatternRepository: failed to acquire database lock");

        let mut stmt = conn.prepare(
            r#"
            SELECT id, pattern_text, category, file_extension, source_count,
                   is_edited, enabled, created_at, updated_at
            FROM learned_patterns
            WHERE id = ?1
            "#,
        )?;

        let result = stmt
            .query_row([id], |row| {
                Ok(LearnedPattern {
                    id: row.get(0)?,
                    pattern_text: row.get(1)?,
                    category: row.get(2)?,
                    file_extension: row.get(3)?,
                    source_count: row.get(4)?,
                    is_edited: row.get::<_, i32>(5)? != 0,
                    enabled: row.get::<_, i32>(6)? != 0,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .optional()?;

        Ok(result)
    }

    /// Create a new pattern (from AI compaction or manual creation)
    pub fn create(&self, input: &LearnedPatternInput, source_count: i32) -> Result<LearnedPattern> {
        let conn = self
            .conn
            .lock()
            .expect("LearnedPatternRepository: failed to acquire database lock");

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let enabled = input.enabled.unwrap_or(true);
        let is_edited = source_count == 0; // Manual creation counts as edited

        conn.execute(
            r#"
            INSERT INTO learned_patterns (
                id, pattern_text, category, file_extension, source_count,
                is_edited, enabled, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            rusqlite::params![
                id,
                input.pattern_text,
                input.category,
                input.file_extension,
                source_count,
                is_edited as i32,
                enabled as i32,
                now,
                now
            ],
        )?;

        Ok(LearnedPattern {
            id,
            pattern_text: input.pattern_text.clone(),
            category: input.category.clone(),
            file_extension: input.file_extension.clone(),
            source_count,
            is_edited,
            enabled,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Update an existing pattern
    pub fn update(&self, id: &str, input: &LearnedPatternInput) -> Result<Option<LearnedPattern>> {
        let conn = self
            .conn
            .lock()
            .expect("LearnedPatternRepository: failed to acquire database lock");

        let now = Utc::now().to_rfc3339();

        // Update and mark as edited
        let rows = conn.execute(
            r#"
            UPDATE learned_patterns
            SET pattern_text = ?2,
                category = ?3,
                file_extension = ?4,
                enabled = COALESCE(?5, enabled),
                is_edited = 1,
                updated_at = ?6
            WHERE id = ?1
            "#,
            rusqlite::params![
                id,
                input.pattern_text,
                input.category,
                input.file_extension,
                input.enabled.map(|e| e as i32),
                now
            ],
        )?;

        if rows == 0 {
            return Ok(None);
        }

        drop(conn);
        self.find_by_id(id)
    }

    /// Toggle pattern enabled status
    pub fn toggle_enabled(&self, id: &str, enabled: bool) -> Result<usize> {
        let conn = self
            .conn
            .lock()
            .expect("LearnedPatternRepository: failed to acquire database lock");

        let now = Utc::now().to_rfc3339();

        let rows = conn.execute(
            r#"
            UPDATE learned_patterns
            SET enabled = ?2, updated_at = ?3
            WHERE id = ?1
            "#,
            rusqlite::params![id, enabled as i32, now],
        )?;

        Ok(rows)
    }

    /// Delete a pattern
    pub fn delete(&self, id: &str) -> Result<usize> {
        let conn = self
            .conn
            .lock()
            .expect("LearnedPatternRepository: failed to acquire database lock");

        let rows = conn.execute("DELETE FROM learned_patterns WHERE id = ?1", [id])?;
        Ok(rows)
    }

    /// Merge a pattern with an existing one (increment source_count)
    pub fn merge_with(&self, existing_id: &str, additional_sources: i32) -> Result<usize> {
        let conn = self
            .conn
            .lock()
            .expect("LearnedPatternRepository: failed to acquire database lock");

        let now = Utc::now().to_rfc3339();

        let rows = conn.execute(
            r#"
            UPDATE learned_patterns
            SET source_count = source_count + ?2,
                updated_at = ?3
            WHERE id = ?1
            "#,
            rusqlite::params![existing_id, additional_sources, now],
        )?;

        Ok(rows)
    }

    /// Get total pattern count
    pub fn count(&self) -> Result<i64> {
        let conn = self
            .conn
            .lock()
            .expect("LearnedPatternRepository: failed to acquire database lock");

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM learned_patterns", [], |row| {
            row.get(0)
        })?;

        Ok(count)
    }

    /// Get enabled pattern count
    pub fn count_enabled(&self) -> Result<i64> {
        let conn = self
            .conn
            .lock()
            .expect("LearnedPatternRepository: failed to acquire database lock");

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM learned_patterns WHERE enabled = 1",
            [],
            |row| row.get(0),
        )?;

        Ok(count)
    }
}

/// Repository for learning state (key-value store for compaction tracking)
pub struct LearningStateRepository {
    conn: DbConn,
}

impl LearningStateRepository {
    pub fn new(conn: DbConn) -> Self {
        Self { conn }
    }

    /// Get a state value
    pub fn get(&self, key: &str) -> Result<Option<String>> {
        let conn = self
            .conn
            .lock()
            .expect("LearningStateRepository: failed to acquire database lock");

        let result = conn
            .query_row(
                "SELECT value FROM learning_state WHERE key = ?1",
                [key],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        Ok(result)
    }

    /// Set a state value
    pub fn set(&self, key: &str, value: &str) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .expect("LearningStateRepository: failed to acquire database lock");

        let now = Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO learning_state (key, value, updated_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = ?3
            "#,
            rusqlite::params![key, value, now],
        )?;

        Ok(())
    }

    /// Get the learning status summary
    pub fn get_status(&self, pattern_repo: &LearnedPatternRepository) -> Result<LearningStatus> {
        let conn = self
            .conn
            .lock()
            .expect("LearningStateRepository: failed to acquire database lock");

        // Get pending rejections count
        let pending_rejections: i64 = conn.query_row(
            "SELECT COUNT(*) FROM feedback_rejections WHERE processed_for_learning = 0",
            [],
            |row| row.get(0),
        )?;

        // Get last compaction time
        let last_compaction_at = conn
            .query_row(
                "SELECT value FROM learning_state WHERE key = 'last_compaction_at'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        // Get compaction threshold
        let threshold_str = conn
            .query_row(
                "SELECT value FROM learning_state WHERE key = 'compaction_threshold'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        let compaction_threshold = threshold_str.and_then(|s| s.parse().ok()).unwrap_or(10);

        drop(conn);

        // Get pattern counts
        let pattern_count = pattern_repo.count()?;
        let enabled_pattern_count = pattern_repo.count_enabled()?;

        Ok(LearningStatus {
            pending_rejections,
            last_compaction_at,
            pattern_count,
            enabled_pattern_count,
            compaction_threshold,
        })
    }
}
