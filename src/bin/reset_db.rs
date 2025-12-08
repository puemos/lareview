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
    let pr_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM pull_requests", [], |row| row.get(0))?;
    let task_count: i64 = conn.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))?;
    let note_count: i64 = conn.query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))?;
    let diff_count: i64 = conn.query_row("SELECT COUNT(*) FROM diffs", [], |row| row.get(0))?;

    println!("Current record counts:");
    println!("  Pull Requests: {}", pr_count);
    println!("  Tasks: {}", task_count);
    println!("  Notes: {}", note_count);
    println!("  Diffs: {}", diff_count);

    // Reset all tables by deleting all records
    conn.execute("DELETE FROM notes", [])?;
    println!("Cleared notes table");

    conn.execute("DELETE FROM diffs", [])?;
    println!("Cleared diffs table");

    conn.execute("DELETE FROM tasks", [])?;
    println!("Cleared tasks table");

    conn.execute("DELETE FROM pull_requests", [])?;
    println!("Cleared pull_requests table");

    // Verify that all tables are empty
    let pr_count_after: i64 =
        conn.query_row("SELECT COUNT(*) FROM pull_requests", [], |row| row.get(0))?;
    let task_count_after: i64 =
        conn.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))?;
    let note_count_after: i64 =
        conn.query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))?;
    let diff_count_after: i64 =
        conn.query_row("SELECT COUNT(*) FROM diffs", [], |row| row.get(0))?;

    println!("\nAfter reset:");
    println!("  Pull Requests: {}", pr_count_after);
    println!("  Tasks: {}", task_count_after);
    println!("  Notes: {}", note_count_after);
    println!("  Diffs: {}", diff_count_after);

    if pr_count_after == 0
        && task_count_after == 0
        && note_count_after == 0
        && diff_count_after == 0
    {
        println!("\nDatabase successfully reset! All records have been deleted.");
    } else {
        eprintln!("\nWarning: Some records still exist in the database.");
    }

    println!("Database location: {}", db_path.display());

    Ok(())
}
