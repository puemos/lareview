use super::*;
use pmcp::ToolHandler;
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;

// Mutex to ensure database tests run sequentially since they share global state via environment variables
static DB_TEST_MUTEX: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn test_return_task_tool_writes_file() {
    let _guard = DB_TEST_MUTEX.lock().unwrap();
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = tmp_dir.path().join("db.sqlite");

    // Set the database path so Database::open() uses our temp path
    // Save original value to restore later
    let original_db_path = std::env::var("LAREVIEW_DB_PATH").ok();
    unsafe {
        std::env::set_var("LAREVIEW_DB_PATH", db_path.to_string_lossy().to_string());
    }

    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let out_path = tmp.path().to_path_buf();
    let run_context_path = tempfile::NamedTempFile::new().expect("temp file");
    let run_context_content = serde_json::json!({
        "review_id": "rev-test",
        "run_id": "run-test",
        "agent_id": "agent-1",
        "input_ref": "diff",
        "diff_text": "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1,1 +1,1 @@\n-old content\n+new content\n",
        "diff_hash": "h",
        "source": { "type": "diff_paste", "diff_hash": "h" },
        "initial_title": "Test Review",
        "created_at": "2024-01-01T00:00:00Z"
    });
    std::fs::write(run_context_path.path(), run_context_content.to_string())
        .expect("write run context");

    let config = Arc::new(ServerConfig {
        tasks_out: Some(out_path.clone()),
        log_file: None,
        run_context: Some(run_context_path.path().to_path_buf()),
    });

    let tool = tool::create_return_task_tool(config);
    let payload = serde_json::json!({
        "id": "x",
        "title": "test",
        "description": "test task",
        "stats": { "risk": "LOW", "tags": ["test"] },
        "diff_refs": [
            {
                "file": "src/a.rs",
                "hunks": [
                    {
                        "old_start": 1,
                        "old_lines": 1,
                        "new_start": 1,
                        "new_lines": 1
                    }
                ]
            }
        ]
    });
    let res = tool
        .handle(
            payload.clone(),
            pmcp::RequestHandlerExtra::new("test".into(), CancellationToken::new()),
        )
        .await
        .expect("tool call ok");
    assert!(res.get("status").and_then(|v| v.as_str()) == Some("ok"));
    assert!(res.get("task_id").is_some());

    // Read and check that task was written as JSONL
    let written = std::fs::read_to_string(tmp.path()).expect("read tmp");
    let lines: Vec<&str> = written.lines().collect();
    assert_eq!(lines.len(), 1); // One task written as JSONL
    let task: serde_json::Value = serde_json::from_str(lines[0]).expect("parse written task");
    assert_eq!(task.get("id").and_then(|v| v.as_str()), Some("x"));

    // Restore original env var
    if let Some(original) = original_db_path {
        unsafe {
            std::env::set_var("LAREVIEW_DB_PATH", original);
        }
    } else {
        unsafe {
            std::env::remove_var("LAREVIEW_DB_PATH");
        }
    }
}

#[tokio::test]
async fn test_return_task_tool_persists_to_db() {
    let _guard = DB_TEST_MUTEX.lock().unwrap();
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = tmp_dir.path().join("db.sqlite");
    let run_context_path = tmp_dir.path().join("run.json");

    // Set the database path so Database::open() uses our temp path
    // Save original value to restore later
    let original_db_path = std::env::var("LAREVIEW_DB_PATH").ok();
    unsafe {
        std::env::set_var("LAREVIEW_DB_PATH", db_path.to_string_lossy().to_string());
    }

    let run_context = serde_json::json!({
        "review_id": "rev-db",
        "run_id": "run-db",
        "agent_id": "agent-1",
        "input_ref": "diff",
        "diff_text": "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1,1 +1,1 @@\n-old content\n+new content\n",
        "diff_hash": "h",
        "source": { "type": "diff_paste", "diff_hash": "h" },
        "initial_title": "Test Review",
        "created_at": "2024-01-01T00:00:00Z"
    });
    std::fs::write(&run_context_path, run_context.to_string()).expect("write run context");

    let config = Arc::new(ServerConfig {
        tasks_out: None,
        log_file: None,
        run_context: Some(run_context_path),
    });

    let tool = tool::create_return_task_tool(config);
    let payload = serde_json::json!({
        "id": "task-123",
        "title": "DB Task",
        "description": "persist me",
        "stats": { "risk": "HIGH", "tags": ["database"] },
        "diff_refs": [
            {
                "file": "src/a.rs",
                "hunks": [
                    {
                        "old_start": 1,
                        "old_lines": 1,
                        "new_start": 1,
                        "new_lines": 1
                    }
                ]
            }
        ]
    });

    let _ = tool
        .handle(
            payload,
            pmcp::RequestHandlerExtra::new("test".into(), CancellationToken::new()),
        )
        .await
        .expect("tool call ok");

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let db = crate::infra::db::Database::open_at(db_path.clone()).expect("open db");
    let repo = crate::infra::db::TaskRepository::new(db.connection());
    let tasks = repo
        .find_by_run(&"run-db".to_string())
        .expect("tasks for run");
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, "task-123");
    assert_eq!(tasks[0].title, "DB Task");
    assert_eq!(tasks[0].status, crate::domain::TaskStatus::Pending);

    // Restore original env var
    if let Some(original) = original_db_path {
        unsafe {
            std::env::set_var("LAREVIEW_DB_PATH", original);
        }
    } else {
        unsafe {
            std::env::remove_var("LAREVIEW_DB_PATH");
        }
    }
}

#[tokio::test]
async fn test_finalize_review_tool_updates_metadata() {
    let _guard = DB_TEST_MUTEX.lock().unwrap();
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = tmp_dir.path().join("db.sqlite");

    // Set the database path so Database::open() uses our temp path
    // Save original value to restore later
    let original_db_path = std::env::var("LAREVIEW_DB_PATH").ok();
    unsafe {
        std::env::set_var("LAREVIEW_DB_PATH", db_path.to_string_lossy().to_string());
    }

    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let out_path = tmp.path().to_path_buf();

    let config = Arc::new(ServerConfig {
        tasks_out: Some(out_path.clone()),
        log_file: None,
        run_context: None,
    });

    let tool = tool::create_finalize_review_tool(config);
    let payload = serde_json::json!({
        "title": "Final Review Title",
        "summary": "This is the summary of the review"
    });
    let res = tool
        .handle(
            payload,
            pmcp::RequestHandlerExtra::new("test".into(), CancellationToken::new()),
        )
        .await
        .expect("tool call ok");
    assert_eq!(
        res,
        serde_json::json!({ "status": "ok", "message": "Review finalized successfully" })
    );

    // Restore original env var
    if let Some(original) = original_db_path {
        unsafe {
            std::env::set_var("LAREVIEW_DB_PATH", original);
        }
    } else {
        unsafe {
            std::env::remove_var("LAREVIEW_DB_PATH");
        }
    }
}

#[tokio::test]
async fn test_multiple_tasks_and_finalize_persists_correctly() {
    let _guard = DB_TEST_MUTEX.lock().unwrap();
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = tmp_dir.path().join("db.sqlite");
    let run_context_path = tmp_dir.path().join("run.json");

    // Set the database path so Database::open() uses our temp path
    // Save original value to restore later
    let original_db_path = std::env::var("LAREVIEW_DB_PATH").ok();
    unsafe {
        std::env::set_var("LAREVIEW_DB_PATH", db_path.to_string_lossy().to_string());
    }

    let run_context = serde_json::json!({
        "review_id": "rev-multi",
        "run_id": "run-multi",
        "agent_id": "agent-1",
        "input_ref": "diff",
        "diff_text": "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1,1 +1,1 @@\n-old content\n+new content\n",
        "diff_hash": "h",
        "source": { "type": "diff_paste", "diff_hash": "h" },
        "initial_title": "Test Review",
        "created_at": "2024-01-01T00:00:00Z"
    });
    std::fs::write(&run_context_path, run_context.to_string()).expect("write run context");

    let config = Arc::new(ServerConfig {
        tasks_out: None,
        log_file: None,
        run_context: Some(run_context_path.clone()),
    });

    let return_task_tool = tool::create_return_task_tool(config.clone());

    // --- Call 1 ---
    let payload1 = serde_json::json!({
        "id": "task-1",
        "title": "First Task",
        "description": "First task description",
        "stats": { "risk": "LOW", "tags": ["one"] },
        "diff_refs": []
    });

    let _ = return_task_tool
        .handle(
            payload1,
            pmcp::RequestHandlerExtra::new("test".into(), CancellationToken::new()),
        )
        .await
        .expect("tool call 1 ok");

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // --- Call 2 ---
    let payload2 = serde_json::json!({
        "id": "task-2",
        "title": "Second Task",
        "description": "Second task description",
        "stats": { "risk": "MEDIUM", "tags": ["two"] },
        "diff_refs": []
    });

    let _ = return_task_tool
        .handle(
            payload2,
            pmcp::RequestHandlerExtra::new("test".into(), CancellationToken::new()),
        )
        .await
        .expect("tool call 2 ok");

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // --- Verify both tasks are saved ---
    let db = crate::infra::db::Database::open_at(db_path.clone()).expect("open db");
    let task_repo = crate::infra::db::TaskRepository::new(db.connection());
    let tasks = task_repo
        .find_by_run(&"run-multi".to_string())
        .expect("tasks for run");
    assert_eq!(tasks.len(), 2);
    assert!(tasks.iter().any(|t| t.id == "task-1"));
    assert!(tasks.iter().any(|t| t.id == "task-2"));

    // --- Finalize the review ---
    let finalize_tool = tool::create_finalize_review_tool(config.clone());
    let finalize_payload = serde_json::json!({
        "title": "Final Multi-Task Review",
        "summary": "This review has two tasks."
    });
    let _ = finalize_tool
        .handle(
            finalize_payload,
            pmcp::RequestHandlerExtra::new("test".into(), CancellationToken::new()),
        )
        .await
        .expect("finalize tool call ok");

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // --- Re-verify tasks and check review metadata ---
    let review_repo = crate::infra::db::ReviewRepository::new(db.connection());
    let review = review_repo
        .find_by_id(&"rev-multi".to_string())
        .expect("find review")
        .expect("review should exist");

    assert_eq!(review.title, "Final Multi-Task Review");
    assert_eq!(
        review.summary,
        Some("This review has two tasks.".to_string())
    );

    let tasks_after_finalize = task_repo
        .find_by_run(&"run-multi".to_string())
        .expect("tasks for run after finalize");
    assert_eq!(tasks_after_finalize.len(), 2);
    assert_eq!(tasks[0].title, "First Task");
    assert_eq!(tasks[1].title, "Second Task");

    // Restore original env var
    if let Some(original) = original_db_path {
        unsafe {
            std::env::set_var("LAREVIEW_DB_PATH", original);
        }
    } else {
        unsafe {
            std::env::remove_var("LAREVIEW_DB_PATH");
        }
    }
}
