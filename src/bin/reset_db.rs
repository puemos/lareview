use rusqlite::Connection;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Determine database path
    let db_path = if let Ok(path) = std::env::var("LAREVIEW_DB_PATH") {
        std::path::PathBuf::from(path)
    } else {
        let cwd = std::env::current_dir().unwrap_or_default();
        cwd.join(".lareview").join("db.sqlite")
    };

    // Check if database exists
    if !db_path.exists() {
        println!("Database does not exist at: {}", db_path.display());
        println!("No reset needed.");
        return Ok(());
    }

    println!("Connecting to database at: {}", db_path.display());

    let conn = Connection::open(&db_path)?;

    // Store initial counts
    let review_count: i64 = conn.query_row("SELECT COUNT(*) FROM reviews", [], |row| row.get(0))?;
    let run_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM review_runs", [], |row| row.get(0))?;
    let task_count: i64 = conn.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))?;
    let thread_count: i64 = conn.query_row("SELECT COUNT(*) FROM threads", [], |row| row.get(0))?;
    let comment_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM comments", [], |row| row.get(0))?;

    println!("Current record counts:");
    println!("  Reviews: {}", review_count);
    println!("  Review Runs: {}", run_count);
    println!("  Tasks: {}", task_count);
    println!("  Threads: {}", thread_count);
    println!("  Comments: {}", comment_count);

    // Reset all tables by deleting all records
    conn.execute("DELETE FROM comments", [])?;
    println!("Cleared comments table");

    conn.execute("DELETE FROM threads", [])?;
    println!("Cleared threads table");

    conn.execute("DELETE FROM tasks", [])?;
    println!("Cleared tasks table");

    conn.execute("DELETE FROM review_runs", [])?;
    println!("Cleared review_runs table");

    conn.execute("DELETE FROM reviews", [])?;
    println!("Cleared reviews table");

    // Verify that all tables are empty
    let review_count_after: i64 =
        conn.query_row("SELECT COUNT(*) FROM reviews", [], |row| row.get(0))?;
    let run_count_after: i64 =
        conn.query_row("SELECT COUNT(*) FROM review_runs", [], |row| row.get(0))?;
    let task_count_after: i64 =
        conn.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))?;
    let thread_count_after: i64 =
        conn.query_row("SELECT COUNT(*) FROM threads", [], |row| row.get(0))?;
    let comment_count_after: i64 =
        conn.query_row("SELECT COUNT(*) FROM comments", [], |row| row.get(0))?;

    println!("\nAfter reset:");
    println!("  Reviews: {}", review_count_after);
    println!("  Review Runs: {}", run_count_after);
    println!("  Tasks: {}", task_count_after);
    println!("  Threads: {}", thread_count_after);
    println!("  Comments: {}", comment_count_after);

    if review_count_after == 0
        && run_count_after == 0
        && task_count_after == 0
        && thread_count_after == 0
        && comment_count_after == 0
    {
        println!("\nDatabase successfully reset! All records have been deleted.");
    } else {
        eprintln!("\nWarning: Some records still exist in the database.");
    }

    println!("Database location: {}", db_path.display());

    Ok(())
}
