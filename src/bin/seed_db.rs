use log::info;
use serde::{Deserialize, Serialize};
use std::fs;

// Simple structs matching the database schema
#[derive(Debug, Serialize, Deserialize)]
struct Review {
    id: String,
    title: String,
    summary: Option<String>,
    source_json: String,
    active_run_id: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReviewRun {
    id: String,
    #[serde(default)]
    review_id: String,
    agent_id: String,
    input_ref: String,
    #[serde(default)]
    diff_text: String,
    diff_hash: String,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReviewTask {
    id: String,
    #[serde(default)]
    run_id: String,
    title: String,
    description: String,
    files: String, // JSON string
    stats: String, // JSON string
    insight: Option<String>,
    diff_refs: Option<String>,
    diagram: Option<String>,
    ai_generated: bool,
    status: String,
    sub_flow: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SeedData {
    review: Review,
    run: ReviewRun,
}

const SEED_REVIEW_JSON: &str = include_str!("../../test_data/seed/review.json");
const SEED_TASKS_JSON: &str = include_str!("../../test_data/seed/tasks.json");
const SEED_DIFF_TEXT: &str = include_str!("../../test_data/seed/calcom_audit.diff");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging - respects RUST_LOG environment variable
    let _ = env_logger::try_init();
    run()
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Determine database path
    let db_path = if let Ok(path) = std::env::var("LAREVIEW_DB_PATH") {
        std::path::PathBuf::from(path)
    } else {
        let cwd = std::env::current_dir().unwrap_or_default();
        cwd.join(".lareview").join("db.sqlite")
    };

    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }

    info!("Connecting to database at: {}", db_path.display());

    let db = lareview::infra::db::Database::open_at(db_path.clone())?;
    let conn = db.connection();
    let conn = conn.lock().unwrap();

    // Parse seed data
    let data: SeedData = serde_json::from_str(SEED_REVIEW_JSON)?;
    let review = data.review;
    let mut review_run = data.run;
    review_run.review_id = review.id.clone();
    review_run.diff_text = SEED_DIFF_TEXT.to_string();

    let tasks: Vec<ReviewTask> = serde_json::from_str(SEED_TASKS_JSON)?;

    // Insert Review
    conn.execute(
        "INSERT OR REPLACE INTO reviews (id, title, summary, source_json, active_run_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (&review.id, &review.title, &review.summary, &review.source_json, &review.active_run_id, &review.created_at, &review.updated_at),
    )?;
    info!("Inserted Review: {}", review.title);

    // Insert Review Run
    conn.execute(
        "INSERT OR REPLACE INTO review_runs (id, review_id, agent_id, input_ref, diff_text, diff_hash, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (&review_run.id, &review_run.review_id, &review_run.agent_id, &review_run.input_ref, &review_run.diff_text, &review_run.diff_hash, &review_run.created_at),
    )?;
    info!("Inserted Review Run for review: {}", review.title);

    // Insert all tasks
    for mut task in tasks {
        task.run_id = review_run.id.clone();
        conn.execute(
            r#"INSERT OR REPLACE INTO tasks (id, run_id, title, description, files, stats, insight, diff_refs, diagram, ai_generated, status, sub_flow) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"#,
            (
                &task.id,
                &task.run_id,
                &task.title,
                &task.description,
                &task.files,
                &task.stats,
                &task.insight,
                &task.diff_refs,
                &task.diagram,
                if task.ai_generated { 1 } else { 0 },
                &task.status,
                &task.sub_flow,
            ),
        )?;
        info!(
            "Inserted task: {} (Sub-flow: {})",
            task.title,
            task.sub_flow.as_deref().unwrap_or("None")
        );
    }

    info!("Sample data successfully added to database!");
    info!("Database location: {}", db_path.display());
    info!(
        "Run the application with `cargo run` to see the intent-centric layout against this booking audit PR."
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use tempfile::NamedTempFile;

    #[test]
    fn test_seed_db_run() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        // Unsafe but safe in test context - single-threaded test execution
        unsafe {
            std::env::set_var("LAREVIEW_DB_PATH", &path);
        }

        run().unwrap();

        let conn = Connection::open(&path).unwrap();
        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM reviews", [], |row| {
                row.get::<_, i32>(0)
            })
            .unwrap();
        assert!(count > 0);

        unsafe {
            std::env::remove_var("LAREVIEW_DB_PATH");
        }
    }
}
