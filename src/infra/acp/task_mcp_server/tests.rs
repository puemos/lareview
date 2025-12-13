use super::*;
use pmcp::ToolHandler;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn test_return_tasks_tool_writes_file() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let out_path = tmp.path().to_path_buf();
    let tmp_db = tempfile::tempdir().expect("tmp db dir");
    let db_path = tmp_db.path().join("db.sqlite");

    let config = Arc::new(ServerConfig {
        tasks_out: Some(out_path.clone()),
        log_file: None,
        run_context: None,
        db_path: Some(db_path),
    });

    let tool = tool::create_return_tasks_tool(config);
    let payload = serde_json::json!({ "tasks": [{ "id": "x", "title": "test" }] });
    let res = tool
        .handle(
            payload.clone(),
            pmcp::RequestHandlerExtra::new("test".into(), CancellationToken::new()),
        )
        .await
        .expect("tool call ok");
    assert_eq!(
        res,
        serde_json::json!({ "status": "ok", "message": "Tasks received successfully" })
    );
    let written = std::fs::read_to_string(tmp.path()).expect("read tmp");
    assert_eq!(written, payload.to_string());
}

#[tokio::test]
async fn test_return_tasks_tool_persists_to_db() {
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = tmp_dir.path().join("db.sqlite");
    let run_context_path = tmp_dir.path().join("run.json");

    let run_context = serde_json::json!({
        "review_id": "rev-db",
        "run_id": "run-db",
        "agent_id": "agent-1",
        "input_ref": "diff",
        "diff_text": "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n",
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
        db_path: Some(db_path.clone()),
    });

    let tool = tool::create_return_tasks_tool(config);
    let payload = serde_json::json!({
        "tasks": [{
            "id": "task-123",
            "title": "DB Task",
            "description": "persist me",
            "stats": { "risk": "HIGH" },
            "diffs": ["@@ -1 +1 @@"]
        }]
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
}
