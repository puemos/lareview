use log::{info, warn};
use rusqlite::Connection;

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

    // Check if database exists
    if !db_path.exists() {
        info!(
            "Database does not exist at: {}. No reset needed.",
            db_path.display()
        );
        return Ok(());
    }

    info!("Connecting to database at: {}", db_path.display());

    let conn = Connection::open(&db_path)?;

    // Store initial counts
    // We use try_query because tables might not exist if it's not initialized
    let tables_exist: i32 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='reviews'",
        [],
        |row| row.get(0),
    )?;

    if tables_exist == 0 {
        info!("Tables do not exist. No reset needed.");
        return Ok(());
    }

    let review_count: i64 = conn.query_row("SELECT COUNT(*) FROM reviews", [], |row| row.get(0))?;
    let run_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM review_runs", [], |row| row.get(0))?;
    let task_count: i64 = conn.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))?;
    let feedback_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM feedback", [], |row| row.get(0))?;
    let comment_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM comments", [], |row| row.get(0))?;

    info!("Current record counts:");
    info!("  Reviews: {}", review_count);
    info!("  Review Runs: {}", run_count);
    info!("  Tasks: {}", task_count);
    info!("  Feedback: {}", feedback_count);
    info!("  Comments: {}", comment_count);

    // Reset all tables by deleting all records
    conn.execute("DELETE FROM comments", [])?;
    info!("Cleared comments table");

    conn.execute("DELETE FROM feedback", [])?;
    info!("Cleared feedback table");

    conn.execute("DELETE FROM tasks", [])?;
    info!("Cleared tasks table");

    conn.execute("DELETE FROM review_runs", [])?;
    info!("Cleared review_runs table");

    conn.execute("DELETE FROM reviews", [])?;
    info!("Cleared reviews table");

    // Verify that all tables are empty
    let review_count_after: i64 =
        conn.query_row("SELECT COUNT(*) FROM reviews", [], |row| row.get(0))?;
    let run_count_after: i64 =
        conn.query_row("SELECT COUNT(*) FROM review_runs", [], |row| row.get(0))?;
    let task_count_after: i64 =
        conn.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))?;
    let feedback_count_after: i64 =
        conn.query_row("SELECT COUNT(*) FROM feedback", [], |row| row.get(0))?;
    let comment_count_after: i64 =
        conn.query_row("SELECT COUNT(*) FROM comments", [], |row| row.get(0))?;

    info!("After reset:");
    info!("  Reviews: {}", review_count_after);
    info!("  Review Runs: {}", run_count_after);
    info!("  Tasks: {}", task_count_after);
    info!("  Feedback: {}", feedback_count_after);
    info!("  Comments: {}", comment_count_after);

    if review_count_after == 0
        && run_count_after == 0
        && task_count_after == 0
        && feedback_count_after == 0
        && comment_count_after == 0
    {
        info!("Database successfully reset! All records have been deleted.");
    } else {
        warn!("Some records still exist in the database.");
    }

    info!("Database location: {}", db_path.display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_reset_db_run() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        // Unsafe but safe in test context - single-threaded test execution
        unsafe {
            std::env::set_var("LAREVIEW_DB_PATH", &path);
        }

        // Use a real database init to create tables first
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch("CREATE TABLE reviews (id TEXT PRIMARY KEY); CREATE TABLE review_runs (id TEXT PRIMARY KEY); CREATE TABLE tasks (id TEXT PRIMARY KEY); CREATE TABLE feedback (id TEXT PRIMARY KEY); CREATE TABLE comments (id TEXT PRIMARY KEY);").unwrap();
            conn.execute("INSERT INTO reviews (id) VALUES ('r1')", [])
                .unwrap();
        }

        run().unwrap();

        let conn = Connection::open(&path).unwrap();
        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM reviews", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);

        unsafe {
            std::env::remove_var("LAREVIEW_DB_PATH");
        }
    }
}
