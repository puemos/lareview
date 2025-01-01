use super::DbConn;
use crate::domain::{ReviewRunId, ReviewStatus, ReviewTask, TaskId};
use anyhow::Result;
use std::sync::Arc;

use std::str::FromStr;

/// Repository for task operations.
pub struct TaskRepository {
    conn: DbConn,
}

impl TaskRepository {
    pub fn new(conn: DbConn) -> Self {
        Self { conn }
    }

    pub fn save(&self, task: &ReviewTask) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let files_json = serde_json::to_string(&task.files)?;
        let stats_json = serde_json::to_string(&task.stats)?;
        let diff_refs_json = serde_json::to_string(&task.diff_refs)?;

        let status_str = task.status.to_string();

        conn.execute(
            r#"
            INSERT OR REPLACE INTO tasks (id, run_id, title, description, files, stats, insight, diff_refs, diagram, ai_generated, status, sub_flow)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            (
                &task.id,
                &task.run_id,
                &task.title,
                &task.description,
                &files_json,
                &stats_json,
                &task.insight,
                &diff_refs_json,
                &task.diagram,
                task.ai_generated as i32,
                &status_str,
                &task.sub_flow,
            ),
        )?;
        Ok(())
    }

    pub fn update_status(&self, task_id: &TaskId, new_status: ReviewStatus) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let status_str = new_status.to_string();
        conn.execute(
            "UPDATE tasks SET status = ?1 WHERE id = ?2",
            (&status_str, task_id),
        )?;
        Ok(())
    }

    pub fn find_by_id(&self, task_id: &TaskId) -> Result<Option<ReviewTask>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, run_id, title, description, files, stats, insight, diff_refs, diagram, ai_generated, status, sub_flow FROM tasks WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map([task_id], |row| {
            let run_id: String = row.get(1)?;
            let files_json: String = row.get(4)?;
            let stats_json: String = row.get(5)?;
            let diff_refs_json: Option<String> = row.get(7)?;
            let status_str: String = row.get(10)?;
            let sub_flow: Option<String> = row.get(11)?;

            Ok(ReviewTask {
                id: row.get::<_, String>(0)?,
                run_id,
                title: row.get::<_, String>(2)?,
                description: row.get::<_, String>(3)?,
                files: serde_json::from_str(&files_json).unwrap_or_default(),
                stats: serde_json::from_str(&stats_json).unwrap_or_default(),
                insight: row.get::<_, Option<String>>(6)?.map(Arc::from),
                diff_refs: diff_refs_json
                    .map(|s| serde_json::from_str(&s).unwrap_or_default())
                    .unwrap_or_default(),
                diagram: row.get::<_, Option<String>>(8)?.map(Arc::from),
                ai_generated: row.get::<_, i32>(9)? != 0,
                status: ReviewStatus::from_str(&status_str).unwrap_or_default(),
                sub_flow,
            })
        })?;

        if let Some(row) = rows.next() {
            row.map(Some).map_err(Into::into)
        } else {
            Ok(None)
        }
    }

    pub fn delete_by_ids(&self, task_ids: &[TaskId]) -> Result<usize> {
        if task_ids.is_empty() {
            return Ok(0);
        }
        let conn = self.conn.lock().unwrap();
        let placeholders = std::iter::repeat_n("?", task_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!("DELETE FROM tasks WHERE id IN ({placeholders})");
        let affected = conn.execute(&sql, rusqlite::params_from_iter(task_ids.iter()))?;
        Ok(affected)
    }

    pub fn find_all(&self) -> Result<Vec<ReviewTask>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, run_id, title, description, files, stats, insight, diff_refs, diagram, ai_generated, status, sub_flow FROM tasks",
        )?;

        let rows = stmt.query_map([], |row| {
            let run_id: String = row.get(1)?;
            let files_json: String = row.get(4)?;
            let stats_json: String = row.get(5)?;
            let diff_refs_json: Option<String> = row.get(7)?;
            let status_str: String = row.get(10)?;
            let sub_flow: Option<String> = row.get(11)?;

            Ok((
                row.get::<_, String>(0)?,
                run_id,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                files_json,
                stats_json,
                row.get::<_, Option<String>>(6)?,
                diff_refs_json,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, i32>(9)?,
                status_str,
                sub_flow,
            ))
        })?;

        let mut tasks = Vec::new();
        for row in rows {
            let (
                id,
                run_id,
                title,
                description,
                files_json,
                stats_json,
                insight,
                diff_refs_json,
                diagram,
                ai_generated,
                status_str,
                sub_flow,
            ) = row?;

            let status = ReviewStatus::from_str(&status_str).unwrap_or_default();

            tasks.push(ReviewTask {
                id,
                run_id,
                title,
                description,
                files: serde_json::from_str(&files_json).unwrap_or_default(),
                stats: serde_json::from_str(&stats_json).unwrap_or_default(),
                insight: insight.map(Arc::from),
                diff_refs: diff_refs_json
                    .map(|s| serde_json::from_str(&s).unwrap_or_default())
                    .unwrap_or_default(),
                diagram: diagram.map(Arc::from),
                ai_generated: ai_generated != 0,
                status,
                sub_flow,
            });
        }
        Ok(tasks)
    }

    pub fn find_by_run_ids(&self, run_ids: &[ReviewRunId]) -> Result<Vec<ReviewTask>> {
        if run_ids.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn.lock().unwrap();
        let placeholders = std::iter::repeat_n("?", run_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT id, run_id, title, description, files, stats, insight, diff_refs, diagram, ai_generated, status, sub_flow FROM tasks WHERE run_id IN ({})",
            placeholders
        );

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(run_ids.iter()), |row| {
            let run_id: String = row.get(1)?;
            let files_json: String = row.get(4)?;
            let stats_json: String = row.get(5)?;
            let diff_refs_json: Option<String> = row.get(7)?;
            let status_str: String = row.get(10)?;
            let sub_flow: Option<String> = row.get(11)?;

            Ok((
                row.get::<_, String>(0)?,
                run_id,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                files_json,
                stats_json,
                row.get::<_, Option<String>>(6)?,
                diff_refs_json,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, i32>(9)?,
                status_str,
                sub_flow,
            ))
        })?;

        let mut tasks = Vec::new();
        for row in rows {
            let (
                id,
                run_id,
                title,
                description,
                files_json,
                stats_json,
                insight,
                diff_refs_json,
                diagram,
                ai_generated,
                status_str,
                sub_flow,
            ) = row?;

            let status = ReviewStatus::from_str(&status_str).unwrap_or_default();

            tasks.push(ReviewTask {
                id,
                run_id,
                title,
                description,
                files: serde_json::from_str(&files_json).unwrap_or_default(),
                stats: serde_json::from_str(&stats_json).unwrap_or_default(),
                insight: insight.map(Arc::from),
                diff_refs: diff_refs_json
                    .map(|s| serde_json::from_str(&s).unwrap_or_default())
                    .unwrap_or_default(),
                diagram: diagram.map(Arc::from),
                ai_generated: ai_generated != 0,
                status,
                sub_flow,
            });
        }
        Ok(tasks)
    }

    #[allow(dead_code)] // Used by ACP modules; invoked indirectly.
    pub fn find_by_run(&self, run_id_filter: &ReviewRunId) -> Result<Vec<ReviewTask>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, run_id, title, description, files, stats, insight, diff_refs, diagram, ai_generated, status, sub_flow FROM tasks WHERE run_id = ?1",
        )?;

        let rows = stmt.query_map([run_id_filter], |row| {
            let run_id: String = row.get(1)?;
            let files_json: String = row.get(4)?;
            let stats_json: String = row.get(5)?;
            let diff_refs_json: Option<String> = row.get(7)?;
            let status_str: String = row.get(10)?;
            let sub_flow: Option<String> = row.get(11)?;

            Ok((
                row.get::<_, String>(0)?,
                run_id,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                files_json,
                stats_json,
                row.get::<_, Option<String>>(6)?,
                diff_refs_json,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, i32>(9)?,
                status_str,
                sub_flow,
            ))
        })?;

        let mut tasks = Vec::new();
        for row in rows {
            let (
                id,
                run_id,
                title,
                description,
                files_json,
                stats_json,
                insight,
                diff_refs_json,
                diagram,
                ai_generated,
                status_str,
                sub_flow,
            ) = row?;

            let status = ReviewStatus::from_str(&status_str).unwrap_or_default();

            tasks.push(ReviewTask {
                id,
                run_id,
                title,
                description,
                files: serde_json::from_str(&files_json).unwrap_or_default(),
                stats: serde_json::from_str(&stats_json).unwrap_or_default(),
                insight: insight.map(Arc::from),
                diff_refs: diff_refs_json
                    .map(|s| serde_json::from_str(&s).unwrap_or_default())
                    .unwrap_or_default(),
                diagram: diagram.map(Arc::from),
                ai_generated: ai_generated != 0,
                status,
                sub_flow,
            });
        }
        Ok(tasks)
    }
}
