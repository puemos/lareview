//! Repository for IssueCheck and IssueFinding persistence

use crate::domain::{CheckStatus, Confidence, FeedbackImpact, IssueCheck, IssueFinding};
use anyhow::{Context, Result};
use rusqlite::{Row, params};
use std::str::FromStr;

use super::{DbConn, Repository};

pub struct IssueCheckRepository {
    conn: DbConn,
}

impl Repository for IssueCheckRepository {}

impl IssueCheckRepository {
    pub fn new(conn: DbConn) -> Self {
        Self { conn }
    }

    /// Save an issue check to the database
    pub fn save(&self, check: &IssueCheck) -> Result<()> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        conn.execute(
            r#"
            INSERT OR REPLACE INTO issue_checks
                (id, run_id, rule_id, category, display_name, status, confidence, summary, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                check.id,
                check.run_id,
                check.rule_id,
                check.category,
                check.display_name,
                check.status.to_string(),
                check.confidence.to_string(),
                check.summary,
                check.created_at
            ],
        )
        .context("save issue check")?;
        Ok(())
    }

    /// Save an issue finding to the database
    pub fn save_finding(&self, finding: &IssueFinding) -> Result<()> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        conn.execute(
            r#"
            INSERT OR REPLACE INTO issue_findings
                (id, check_id, title, description, evidence, file_path, line_number, impact, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                finding.id,
                finding.check_id,
                finding.title,
                finding.description,
                finding.evidence,
                finding.file_path,
                finding.line_number.map(|n| n as i64),
                finding.impact.to_string(),
                finding.created_at
            ],
        )
        .context("save issue finding")?;
        Ok(())
    }

    /// Find all issue checks for a review run
    pub fn find_by_run(&self, run_id: &str) -> Result<Vec<IssueCheck>> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        let mut stmt = conn.prepare(
            r#"
            SELECT id, run_id, rule_id, category, display_name, status, confidence, summary, created_at
            FROM issue_checks
            WHERE run_id = ?1
            ORDER BY category ASC
            "#,
        )?;
        let rows = stmt.query_map([run_id], Self::row_to_check)?;
        let mut checks = Vec::new();
        for row in rows {
            checks.push(row?);
        }
        Ok(checks)
    }

    /// Find all findings for an issue check
    pub fn find_findings_by_check(&self, check_id: &str) -> Result<Vec<IssueFinding>> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        let mut stmt = conn.prepare(
            r#"
            SELECT id, check_id, title, description, evidence, file_path, line_number, impact, created_at
            FROM issue_findings
            WHERE check_id = ?1
            ORDER BY created_at ASC
            "#,
        )?;
        let rows = stmt.query_map([check_id], Self::row_to_finding)?;
        let mut findings = Vec::new();
        for row in rows {
            findings.push(row?);
        }
        Ok(findings)
    }

    /// Find an issue check by ID
    pub fn find_by_id(&self, id: &str) -> Result<Option<IssueCheck>> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        let mut stmt = conn.prepare(
            r#"
            SELECT id, run_id, rule_id, category, display_name, status, confidence, summary, created_at
            FROM issue_checks
            WHERE id = ?1
            "#,
        )?;
        let mut rows = stmt.query_map([id], Self::row_to_check)?;
        if let Some(row) = rows.next() {
            return Ok(Some(row?));
        }
        Ok(None)
    }

    /// Delete all issue checks (and findings via CASCADE) for a run
    pub fn delete_by_run(&self, run_id: &str) -> Result<usize> {
        let conn = self.conn.lock().expect("Failed to acquire database lock");
        let affected = conn.execute("DELETE FROM issue_checks WHERE run_id = ?1", [run_id])?;
        Ok(affected)
    }

    /// Get checks with their findings for a run
    pub fn find_checks_with_findings(
        &self,
        run_id: &str,
    ) -> Result<Vec<(IssueCheck, Vec<IssueFinding>)>> {
        let checks = self.find_by_run(run_id)?;
        let mut result = Vec::new();
        for check in checks {
            let findings = self.find_findings_by_check(&check.id)?;
            result.push((check, findings));
        }
        Ok(result)
    }

    fn row_to_check(row: &Row<'_>) -> rusqlite::Result<IssueCheck> {
        let status_str: String = row.get(5)?;
        let status = CheckStatus::from_str(&status_str).unwrap_or(CheckStatus::NotFound);
        let confidence_str: String = row.get(6)?;
        let confidence = Confidence::from_str(&confidence_str).unwrap_or(Confidence::High);

        Ok(IssueCheck {
            id: row.get(0)?,
            run_id: row.get(1)?,
            rule_id: row.get(2)?,
            category: row.get(3)?,
            display_name: row.get(4)?,
            status,
            confidence,
            summary: row.get(7)?,
            created_at: row.get(8)?,
        })
    }

    fn row_to_finding(row: &Row<'_>) -> rusqlite::Result<IssueFinding> {
        let impact_str: String = row.get(7)?;
        let impact = FeedbackImpact::from_str(&impact_str).unwrap_or(FeedbackImpact::Nitpick);
        let line_number: Option<i64> = row.get(6)?;

        Ok(IssueFinding {
            id: row.get(0)?,
            check_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            evidence: row.get(4)?,
            file_path: row.get(5)?,
            line_number: line_number.map(|n| n as u32),
            impact,
            created_at: row.get(8)?,
        })
    }
}
