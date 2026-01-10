use crate::domain::{ReviewRule, RuleScope};
use anyhow::{Context, Result};
use rusqlite::{Row, params};
use std::str::FromStr;

use super::{DbConn, Repository};

pub struct ReviewRuleRepository {
    conn: DbConn,
}

impl Repository for ReviewRuleRepository {}

impl ReviewRuleRepository {
    pub fn new(conn: DbConn) -> Self {
        Self { conn }
    }

    pub fn save(&self, rule: &ReviewRule) -> Result<()> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        conn.execute(
            r#"
            INSERT OR REPLACE INTO review_rules
                (id, scope, repo_id, glob, text, enabled, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                rule.id,
                rule.scope.to_string(),
                rule.repo_id,
                rule.glob,
                rule.text,
                if rule.enabled { 1 } else { 0 },
                rule.created_at,
                rule.updated_at
            ],
        )
        .context("save review rule")?;
        Ok(())
    }

    pub fn list_all(&self) -> Result<Vec<ReviewRule>> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        let mut stmt = conn.prepare(
            r#"
            SELECT id, scope, repo_id, glob, text, enabled, created_at, updated_at
            FROM review_rules
            ORDER BY created_at DESC
            "#,
        )?;
        let rows = stmt.query_map([], Self::row_to_rule)?;
        let mut rules = Vec::new();
        for row in rows {
            rules.push(row?);
        }
        Ok(rules)
    }

    pub fn list_enabled(&self) -> Result<Vec<ReviewRule>> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        let mut stmt = conn.prepare(
            r#"
            SELECT id, scope, repo_id, glob, text, enabled, created_at, updated_at
            FROM review_rules
            WHERE enabled = 1
            ORDER BY created_at DESC
            "#,
        )?;
        let rows = stmt.query_map([], Self::row_to_rule)?;
        let mut rules = Vec::new();
        for row in rows {
            rules.push(row?);
        }
        Ok(rules)
    }

    pub fn find_by_id(&self, id: &str) -> Result<Option<ReviewRule>> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        let mut stmt = conn.prepare(
            r#"
            SELECT id, scope, repo_id, glob, text, enabled, created_at, updated_at
            FROM review_rules
            WHERE id = ?1
            "#,
        )?;
        let mut rows = stmt.query_map([id], Self::row_to_rule)?;
        if let Some(row) = rows.next() {
            return Ok(Some(row?));
        }
        Ok(None)
    }

    pub fn delete(&self, id: &str) -> Result<usize> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        let affected = conn.execute("DELETE FROM review_rules WHERE id = ?1", [id])?;
        Ok(affected)
    }

    fn row_to_rule(row: &Row<'_>) -> rusqlite::Result<ReviewRule> {
        let scope_str: String = row.get(1)?;
        let scope = RuleScope::from_str(&scope_str).unwrap_or(RuleScope::Global);
        let enabled: i64 = row.get(5)?;
        Ok(ReviewRule {
            id: row.get(0)?,
            scope,
            repo_id: row.get(2)?,
            glob: row.get(3)?,
            text: row.get(4)?,
            enabled: enabled != 0,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    }
}
