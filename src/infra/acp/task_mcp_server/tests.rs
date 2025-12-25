use super::*;
use pmcp::ToolHandler;
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;

// Mutex to ensure database tests run sequentially since they share global state via environment variables
static DB_TEST_MUTEX: Mutex<()> = Mutex::new(());

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_return_task_tool_writes_file() {
    // Acquire lock for the duration of the test
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
        repo_root: None,
        db_path: Some(db_path.clone()),
    });

    let tool = tool::create_return_task_tool(config);
    let payload = serde_json::json!({
        "id": "x",
        "title": "test",
        "description": "test task",
        "stats": { "risk": "LOW", "tags": ["test"] },
        "diagram": "sequence_diagram: { Reviewer -> Code: \"review\" }",
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
#[allow(clippy::await_holding_lock)]
async fn test_return_task_tool_persists_to_db() {
    // Acquire lock for the duration of the test
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
        repo_root: None,
        db_path: Some(db_path.clone()),
    });

    let tool = tool::create_return_task_tool(config);
    let payload = serde_json::json!({
        "id": "task-123",
        "title": "DB Task",
        "description": "persist me",
        "stats": { "risk": "HIGH", "tags": ["database"] },
        "diagram": "sequence_diagram: { Reviewer -> DB: \"check\" }",
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
    assert_eq!(tasks[0].status, crate::domain::ReviewStatus::Todo);

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
#[allow(clippy::await_holding_lock)]
async fn test_finalize_review_tool_updates_metadata() {
    // Acquire lock for the duration of the test
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
        repo_root: None,
        db_path: Some(db_path.clone()),
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
#[allow(clippy::await_holding_lock)]
async fn test_multiple_tasks_and_finalize_persists_correctly() {
    // Acquire lock for the duration of the test
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
        repo_root: None,
        db_path: Some(db_path.clone()),
    });

    let return_task_tool = tool::create_return_task_tool(config.clone());

    // --- Call 1 ---
    let payload1 = serde_json::json!({
        "id": "task-1",
        "title": "First Task",
        "description": "First task description",
        "stats": { "risk": "LOW", "tags": ["one"] },
        "diagram": "sequence_diagram: { Reviewer -> Code: \"review\" }",
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
        "diagram": "sequence_diagram: { Reviewer -> Code: \"verify\" }",
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
async fn test_repo_list_files_tool() {
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let repo_root = tmp_dir.path().to_path_buf();

    // Create some files
    std::fs::write(repo_root.join("file1.rs"), "content1").unwrap();
    std::fs::create_dir(repo_root.join("subdir")).unwrap();
    std::fs::write(repo_root.join("subdir/file2.ts"), "content2").unwrap();

    let config = Arc::new(ServerConfig {
        tasks_out: None,
        log_file: None,
        run_context: None,
        repo_root: Some(repo_root.clone()),
        db_path: None,
    });

    let tool = tool::create_repo_list_files_tool(config);
    let payload = serde_json::json!(
        {
            "path": ".",
            "include_dirs": true
        }
    );

    let res = tool
        .handle(
            payload,
            pmcp::RequestHandlerExtra::new("test".into(), CancellationToken::new()),
        )
        .await
        .unwrap();
    let entries = res.get("entries").unwrap().as_array().unwrap();

    assert!(
        entries
            .iter()
            .any(|e| e.get("path").unwrap().as_str() == Some("file1.rs"))
    );
    assert!(
        entries
            .iter()
            .any(|e| e.get("path").unwrap().as_str() == Some("subdir"))
    );
}

#[tokio::test]
async fn test_repo_search_tool() {
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let repo_root = tmp_dir.path().to_path_buf();

    std::fs::write(repo_root.join("search.rs"), "fn find_me() {}").unwrap();

    let config = Arc::new(ServerConfig {
        tasks_out: None,
        log_file: None,
        run_context: None,
        repo_root: Some(repo_root.clone()),
        db_path: None,
    });

    let tool = tool::create_repo_search_tool(config);
    let payload = serde_json::json!(
        {
            "query": "find_me"
        }
    );

    let res = tool
        .handle(
            payload,
            pmcp::RequestHandlerExtra::new("test".into(), CancellationToken::new()),
        )
        .await
        .unwrap();
    let matches = res.get("matches").unwrap().as_array().unwrap();

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].get("path").unwrap().as_str(), Some("search.rs"));
    assert!(
        matches[0]
            .get("text")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("find_me")
    );
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_save_agent_comment() {
    let _guard = DB_TEST_MUTEX.lock().unwrap();
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = tmp_dir.path().join("db.sqlite");
    let run_context_path = tmp_dir.path().join("run.json");

    unsafe {
        std::env::set_var("LAREVIEW_DB_PATH", db_path.to_string_lossy().to_string());
    }

    let run_context = serde_json::json!({
        "review_id": "rev-1",
        "run_id": "run-1",
        "agent_id": "agent-1",
        "input_ref": "diff",
        "diff_text": "diff --git a/file.rs b/file.rs\n--- a/file.rs\n+++ b/file.rs\n@@ -1,1 +1,1 @@\n-old\n+new\n",
        "diff_hash": "h",
        "source": { "type": "diff_paste", "diff_hash": "h" },
        "initial_title": "Test",
        "created_at": "2024-01-01T00:00:00Z"
    });
    std::fs::write(&run_context_path, run_context.to_string()).unwrap();

    let config = ServerConfig {
        tasks_out: None,
        log_file: None,
        run_context: Some(run_context_path),
        repo_root: None,
        db_path: Some(db_path.clone()),
    };

    // First, create a review and run and task so auto-linking works and FKs are happy
    let db = crate::infra::db::Database::open_at(db_path.clone()).expect("open db");

    let review_repo = crate::infra::db::ReviewRepository::new(db.connection());
    let review = crate::domain::Review {
        id: "rev-1".into(),
        title: "Test".into(),
        summary: None,
        source: crate::domain::ReviewSource::DiffPaste {
            diff_hash: "h".into(),
        },
        active_run_id: Some("run-1".into()),
        created_at: "2024-01-01T00:00:00Z".into(),
        updated_at: "2024-01-01T00:00:00Z".into(),
    };
    review_repo.save(&review).unwrap();

    let run_repo = crate::infra::db::ReviewRunRepository::new(db.connection());
    let run = crate::domain::ReviewRun {
            id: "run-1".into(),
            review_id: "rev-1".into(),
            agent_id: "agent-1".into(),
            input_ref: "diff".into(),
            diff_text: "diff --git a/file.rs b/file.rs\n--- a/file.rs\n+++ b/file.rs\n@@ -1,1 +1,1 @@\n-old\n+new\n".into(),
            diff_hash: "h".into(),
            created_at: "2024-01-01T00:00:00Z".into(),
        };
    run_repo.save(&run).unwrap();

    let task_repo = crate::infra::db::TaskRepository::new(db.connection());
    let task = crate::domain::ReviewTask {
        id: "task-1".into(),
        run_id: "run-1".into(),
        title: "Task 1".into(),
        description: "Desc".into(),
        files: vec!["file.rs".into()],
        stats: crate::domain::TaskStats::default(),
        diff_refs: vec![crate::domain::DiffRef {
            file: "file.rs".into(),
            hunks: vec![crate::domain::HunkRef {
                old_start: 1,
                old_lines: 1,
                new_start: 1,
                new_lines: 1,
            }],
        }],
        insight: None,
        diagram: None,
        ai_generated: true,
        status: crate::domain::ReviewStatus::Todo,
        sub_flow: None,
    };
    task_repo.save(&task).unwrap();

    let args = serde_json::json!({
        "file": "file.rs",
        "line": 1,
        "body": "This is a comment",
        "side": "new",
        "impact": "blocking"
    });

    let thread_id =
        crate::infra::acp::task_mcp_server::comment_ingest::save_agent_comment(&config, args)
            .unwrap();
    assert!(!thread_id.is_empty());

    let thread_repo = crate::infra::db::ThreadRepository::new(db.connection());
    let thread = thread_repo.find_by_id(&thread_id).unwrap().unwrap();
    assert_eq!(thread.task_id, Some("task-1".to_string()));
    assert_eq!(thread.impact, crate::domain::ThreadImpact::Blocking);

    let comment_repo = crate::infra::db::CommentRepository::new(db.connection());
    let comments = comment_repo.list_for_thread(&thread_id).unwrap();
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].body, "This is a comment");

    unsafe {
        std::env::remove_var("LAREVIEW_DB_PATH");
    }
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_save_agent_comment_no_task() {
    let _guard = DB_TEST_MUTEX.lock().unwrap();
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = tmp_dir.path().join("db.sqlite");
    let run_context_path = tmp_dir.path().join("run.json");

    unsafe {
        std::env::set_var("LAREVIEW_DB_PATH", db_path.to_string_lossy().to_string());
    }

    let run_context = serde_json::json!({
        "review_id": "rev-1",
        "run_id": "run-1",
        "agent_id": "agent-1",
        "input_ref": "diff",
        "diff_text": "diff --git a/file.rs b/file.rs\n--- a/file.rs\n+++ b/file.rs\n@@ -1,1 +1,1 @@\n-old\n+new\n",
        "diff_hash": "h",
        "source": { "type": "diff_paste", "diff_hash": "h" },
        "initial_title": "Test",
        "created_at": "2024-01-01T00:00:00Z"
    });
    std::fs::write(&run_context_path, run_context.to_string()).unwrap();

    let config = ServerConfig {
        tasks_out: None,
        log_file: None,
        run_context: Some(run_context_path),
        repo_root: None,
        db_path: Some(db_path.clone()),
    };

    let args = serde_json::json!({
        "file": "file.rs",
        "line": 1,
        "body": "Comment without task",
        "side": "new"
    });

    // Should fail because no task covers this line
    let result =
        crate::infra::acp::task_mcp_server::comment_ingest::save_agent_comment(&config, args);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("does not fall within the scope of any generated task")
    );

    unsafe {
        std::env::remove_var("LAREVIEW_DB_PATH");
    }
}
