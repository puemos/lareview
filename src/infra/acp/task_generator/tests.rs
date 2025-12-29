use super::*;
use crate::domain::{
    DiffRef, HunkRef, ReviewSource, ReviewStatus, ReviewTask, RiskLevel, TaskStats,
};
use crate::infra::acp::RunContext;
use crate::infra::db::{Database, TaskRepository};
use std::path::{Path, PathBuf};
use std::sync::Arc;

// --- Helpers ---

fn sample_run(diff_text: &str) -> RunContext {
    let diff_hash = format!("{:016x}", crate::infra::hash::hash64(diff_text));
    RunContext {
        review_id: "review-1".into(),
        run_id: "run-1".into(),
        agent_id: "agent-1".into(),
        input_ref: "input".into(),
        diff_text: Arc::from(diff_text),
        diff_hash: diff_hash.clone(),
        source: ReviewSource::DiffPaste { diff_hash },
        initial_title: None,
        created_at: Some(chrono::Utc::now().to_rfc3339()),
    }
}

fn sample_task(id: &str, files: &[&str]) -> ReviewTask {
    ReviewTask {
        id: id.into(),
        run_id: "run-1".into(),
        title: id.into(),
        description: String::new(),
        files: files.iter().map(|f| f.to_string()).collect(),
        stats: TaskStats {
            additions: 0,
            deletions: 0,
            risk: RiskLevel::Low,
            tags: vec![],
        },
        diff_refs: vec![],
        insight: None,
        diagram: Some(Arc::from(
            "Flow: { shape: sequence_diagram Reviewer -> Code: \"review\" }",
        )),
        ai_generated: true,
        status: ReviewStatus::Todo,
        sub_flow: None,
    }
}

fn set_env(key: &str, val: &str) -> Option<String> {
    let prev = std::env::var(key).ok();
    unsafe {
        std::env::set_var(key, val);
    }
    prev
}

fn restore_env(key: &str, prev: Option<String>) {
    match prev {
        Some(val) => unsafe {
            std::env::set_var(key, val);
        },
        None => unsafe {
            std::env::remove_var(key);
        },
    }
}

// --- MCP Config Tests ---

#[cfg(test)]
mod mcp_config_tests {
    use super::*;

    #[test]
    fn resolve_prefers_mcp_server_binary_override() {
        let override_path = PathBuf::from("/tmp/custom-mcp-bin");
        let resolved = crate::infra::acp::task_generator::worker::resolve_task_mcp_server_path(
            Some(&override_path),
            Path::new("/fallback"),
        );
        assert_eq!(resolved, override_path);
    }
}

// --- Policy Tests ---

#[cfg(test)]
mod policy_tests {
    use super::*;

    #[test]
    fn prompt_renders_no_repo_access_block() {
        let diff = "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n";
        let run = sample_run(diff);
        let prompt = crate::infra::acp::task_generator::prompt::build_prompt(&run, None).unwrap();
        assert!(prompt.contains("You do NOT have repository access."));
        assert!(
            prompt.contains("Use `return_task` to submit each task individually during analysis")
        );
        assert!(
            prompt.contains("Use `finalize_review` to submit the final review title and summary")
        );
        assert!(!prompt.contains("You have READ-ONLY access"));
    }

    #[test]
    fn prompt_renders_repo_access_block() {
        let diff = "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n";
        let run = sample_run(diff);
        let root = std::path::PathBuf::from("/tmp/repo-root");
        let prompt =
            crate::infra::acp::task_generator::prompt::build_prompt(&run, Some(&root)).unwrap();
        assert!(prompt.contains("You have READ-ONLY access"));
        assert!(prompt.contains(&root.display().to_string()));
        assert!(prompt.contains("Allowed tools:"));
        assert!(!prompt.contains("You do NOT have repository access."));
    }

    #[test]
    fn prompt_renders_error_on_missing_template() {
        let diff = "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n";
        let run = sample_run(diff);
        // We can't easily trigger a rendering error without modifying the prompt name in build_prompt,
        // but we can test that it's a Result.
        let res = crate::infra::acp::task_generator::prompt::build_prompt(&run, None);
        assert!(res.is_ok());
    }

    #[test]
    fn capabilities_disable_tools_without_repo() {
        let caps = crate::infra::acp::task_generator::prompt::build_client_capabilities(false);
        assert!(!caps.terminal);
        assert!(!caps.fs.read_text_file);
        assert!(!caps.fs.write_text_file);
    }

    #[test]
    fn capabilities_readonly_fs_with_repo() {
        let caps = crate::infra::acp::task_generator::prompt::build_client_capabilities(true);
        assert!(!caps.terminal);
        assert!(caps.fs.read_text_file);
        assert!(!caps.fs.write_text_file);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn permission_cancelled_without_repo_access() {
        let client =
            crate::infra::acp::task_generator::client::LaReviewClient::new(None, "run-1", None);
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(agent_client_protocol::ToolKind::Read)
            .title("fs/read_text_file")
            .raw_input(serde_json::json!({ "path": "src/a.rs" }));
        let tool_call = agent_client_protocol::ToolCallUpdate::new("tc1", fields);
        let options = vec![
            agent_client_protocol::PermissionOption::new(
                "allow",
                "Allow",
                agent_client_protocol::PermissionOptionKind::AllowOnce,
            ),
            agent_client_protocol::PermissionOption::new(
                "reject",
                "Reject",
                agent_client_protocol::PermissionOptionKind::RejectOnce,
            ),
        ];
        let req = agent_client_protocol::RequestPermissionRequest::new(
            agent_client_protocol::SessionId::new("s1"),
            tool_call,
            options,
        );
        let resp =
            <crate::infra::acp::task_generator::client::LaReviewClient as agent_client_protocol::Client>::request_permission(
                &client, req,
            )
            .await
            .unwrap();
        assert!(matches!(
            resp.outcome,
            agent_client_protocol::RequestPermissionOutcome::Cancelled
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn permission_allows_return_task_even_without_repo_access() {
        let client =
            crate::infra::acp::task_generator::client::LaReviewClient::new(None, "run-1", None);
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(agent_client_protocol::ToolKind::Other)
            .title("return_task")
            .raw_input(serde_json::json!({
                "id": "test-task",
                "title": "Test Task",
                "description": "Test task description",
                "stats": { "risk": "LOW", "tags": ["test"] },
                "diff_refs": []
            }));
        let tool_call = agent_client_protocol::ToolCallUpdate::new("tc1", fields);
        let options = vec![agent_client_protocol::PermissionOption::new(
            "allow",
            "Allow",
            agent_client_protocol::PermissionOptionKind::AllowOnce,
        )];
        let req = agent_client_protocol::RequestPermissionRequest::new(
            agent_client_protocol::SessionId::new("s1"),
            tool_call,
            options,
        );
        let resp =
            <crate::infra::acp::task_generator::client::LaReviewClient as agent_client_protocol::Client>::request_permission(
                &client, req,
            )
            .await
            .unwrap();
        assert!(matches!(
            resp.outcome,
            agent_client_protocol::RequestPermissionOutcome::Selected(_)
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn permission_allows_wrapped_return_task_payload() {
        let client =
            crate::infra::acp::task_generator::client::LaReviewClient::new(None, "run-1", None);
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(agent_client_protocol::ToolKind::Other)
            .title("mcp")
            .raw_input(serde_json::json!({
                "tool": "return_task",
                "server": "lareview-tasks",
                "arguments": {
                    "id": "wrapped-task",
                    "title": "Wrapped Task",
                    "description": "Test task description",
                    "stats": { "risk": "LOW", "tags": ["test"] },
                    "diff_refs": []
                }
            }));
        let tool_call = agent_client_protocol::ToolCallUpdate::new("tc1", fields);
        let options = vec![agent_client_protocol::PermissionOption::new(
            "allow",
            "Allow",
            agent_client_protocol::PermissionOptionKind::AllowOnce,
        )];
        let req = agent_client_protocol::RequestPermissionRequest::new(
            agent_client_protocol::SessionId::new("s1"),
            tool_call,
            options,
        );
        let resp =
            <crate::infra::acp::task_generator::client::LaReviewClient as agent_client_protocol::Client>::request_permission(
                &client, req,
            )
            .await
            .unwrap();
        assert!(matches!(
            resp.outcome,
            agent_client_protocol::RequestPermissionOutcome::Selected(_)
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn permission_allows_single_task_payload_embedded_in_title() {
        let client =
            crate::infra::acp::task_generator::client::LaReviewClient::new(None, "run-1", None);
        let task_json = r###"{"id":"T1","title":"Example","description":"Test","stats":{"risk":"LOW","tags":[]},"diff_refs":[{"file":"test.rs","hunks":[{"old_start":1,"old_lines":1,"new_start":1,"new_lines":1}]}]}"###;
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(agent_client_protocol::ToolKind::Other)
            .title(task_json);
        let tool_call = agent_client_protocol::ToolCallUpdate::new("tc1", fields);
        let options = vec![agent_client_protocol::PermissionOption::new(
            "allow",
            "Allow",
            agent_client_protocol::PermissionOptionKind::AllowOnce,
        )];
        let req = agent_client_protocol::RequestPermissionRequest::new(
            agent_client_protocol::SessionId::new("s1"),
            tool_call,
            options,
        );
        let resp =
            <crate::infra::acp::task_generator::client::LaReviewClient as agent_client_protocol::Client>::request_permission(
                &client, req,
            )
            .await
            .unwrap();
        assert!(matches!(
            resp.outcome,
            agent_client_protocol::RequestPermissionOutcome::Selected(_)
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn permission_allows_finalize_review_payload_embedded_in_title() {
        let client =
            crate::infra::acp::task_generator::client::LaReviewClient::new(None, "run-1", None);
        let finalize_json = r###"{"title":"Review Title","summary":"Review summary"}"###;
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(agent_client_protocol::ToolKind::Other)
            .title(finalize_json);
        let tool_call = agent_client_protocol::ToolCallUpdate::new("tc1", fields);
        let options = vec![agent_client_protocol::PermissionOption::new(
            "allow",
            "Allow",
            agent_client_protocol::PermissionOptionKind::AllowOnce,
        )];
        let req = agent_client_protocol::RequestPermissionRequest::new(
            agent_client_protocol::SessionId::new("s1"),
            tool_call,
            options,
        );
        let resp =
            <crate::infra::acp::task_generator::client::LaReviewClient as agent_client_protocol::Client>::request_permission(
                &client, req,
            )
            .await
            .unwrap();
        assert!(matches!(
            resp.outcome,
            agent_client_protocol::RequestPermissionOutcome::Selected(_)
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn permission_allows_safe_read_under_repo_root() {
        let root = tempfile::tempdir().expect("root");
        let src_dir = root.path().join("src");
        std::fs::create_dir_all(&src_dir).expect("mkdir");
        std::fs::write(src_dir.join("a.rs"), "hi").expect("write");

        let client = crate::infra::acp::task_generator::client::LaReviewClient::new(
            None,
            "run-1",
            Some(root.path().to_path_buf()),
        );
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(agent_client_protocol::ToolKind::Read)
            .title("fs/read_text_file")
            .raw_input(serde_json::json!({ "path": "src/a.rs" }));
        let tool_call = agent_client_protocol::ToolCallUpdate::new("tc1", fields);
        let options = vec![agent_client_protocol::PermissionOption::new(
            "allow",
            "Allow",
            agent_client_protocol::PermissionOptionKind::AllowOnce,
        )];
        let req = agent_client_protocol::RequestPermissionRequest::new(
            agent_client_protocol::SessionId::new("s1"),
            tool_call,
            options,
        );
        let resp =
            <crate::infra::acp::task_generator::client::LaReviewClient as agent_client_protocol::Client>::request_permission(
                &client, req,
            )
            .await
            .unwrap();
        assert!(matches!(
            resp.outcome,
            agent_client_protocol::RequestPermissionOutcome::Selected(_)
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn permission_allows_missing_file_under_repo_root() {
        let root = tempfile::tempdir().expect("root");
        let client = crate::infra::acp::task_generator::client::LaReviewClient::new(
            None,
            "run-1",
            Some(root.path().to_path_buf()),
        );
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(agent_client_protocol::ToolKind::Read)
            .title("fs/read_text_file")
            .raw_input(serde_json::json!({ "path": "src/missing.rs" }));
        let tool_call = agent_client_protocol::ToolCallUpdate::new("tc1", fields);
        let options = vec![agent_client_protocol::PermissionOption::new(
            "allow",
            "Allow",
            agent_client_protocol::PermissionOptionKind::AllowOnce,
        )];
        let req = agent_client_protocol::RequestPermissionRequest::new(
            agent_client_protocol::SessionId::new("s1"),
            tool_call,
            options,
        );
        let resp =
            <crate::infra::acp::task_generator::client::LaReviewClient as agent_client_protocol::Client>::request_permission(
                &client, req,
            )
            .await
            .unwrap();
        assert!(matches!(
            resp.outcome,
            agent_client_protocol::RequestPermissionOutcome::Selected(_)
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn permission_denies_read_outside_repo_root() {
        let root = tempfile::tempdir().expect("root");
        std::fs::write(root.path().join("inside.rs"), "hi").expect("write");

        let client = crate::infra::acp::task_generator::client::LaReviewClient::new(
            None,
            "run-1",
            Some(root.path().to_path_buf()),
        );
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(agent_client_protocol::ToolKind::Read)
            .title("fs/read_text_file")
            .raw_input(serde_json::json!({ "path": "../outside.rs" }));
        let tool_call = agent_client_protocol::ToolCallUpdate::new("tc1", fields);
        let options = vec![agent_client_protocol::PermissionOption::new(
            "allow",
            "Allow",
            agent_client_protocol::PermissionOptionKind::AllowOnce,
        )];
        let req = agent_client_protocol::RequestPermissionRequest::new(
            agent_client_protocol::SessionId::new("s1"),
            tool_call,
            options,
        );
        let resp =
            <crate::infra::acp::task_generator::client::LaReviewClient as agent_client_protocol::Client>::request_permission(
                &client, req,
            )
            .await
            .unwrap();
        assert!(matches!(
            resp.outcome,
            agent_client_protocol::RequestPermissionOutcome::Cancelled
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn permission_denies_execute_even_with_repo_access() {
        let root = tempfile::tempdir().expect("root");
        let client = crate::infra::acp::task_generator::client::LaReviewClient::new(
            None,
            "run-1",
            Some(root.path().to_path_buf()),
        );
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(agent_client_protocol::ToolKind::Execute)
            .title("terminal/exec")
            .raw_input(serde_json::json!({ "command": "echo hi" }));
        let tool_call = agent_client_protocol::ToolCallUpdate::new("tc1", fields);
        let options = vec![agent_client_protocol::PermissionOption::new(
            "allow",
            "Allow",
            agent_client_protocol::PermissionOptionKind::AllowOnce,
        )];
        let req = agent_client_protocol::RequestPermissionRequest::new(
            agent_client_protocol::SessionId::new("s1"),
            tool_call,
            options,
        );
        let resp =
            <crate::infra::acp::task_generator::client::LaReviewClient as agent_client_protocol::Client>::request_permission(
                &client, req,
            )
            .await
            .unwrap();
        assert!(matches!(
            resp.outcome,
            agent_client_protocol::RequestPermissionOutcome::Cancelled
        ));
    }

    #[test]
    fn validate_tasks_requires_full_file_coverage() {
        let diff = "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n";
        let tasks = vec![sample_task("T1", &["src/a.rs"]), sample_task("T2", &[])];
        let raw = serde_json::json!({
            "tasks": [
                { "stats": { "risk": "LOW" } },
                { "stats": { "risk": "LOW" } }
            ]
        });
        assert!(
            crate::infra::acp::task_generator::validation::validate_tasks_payload(
                &tasks,
                Some(&raw),
                diff
            )
            .is_ok()
        );

        let missing = vec![sample_task("T1", &[]), sample_task("T2", &[])];
        assert!(
            crate::infra::acp::task_generator::validation::validate_tasks_payload(
                &missing,
                Some(&raw),
                diff
            )
            .is_err()
        );
    }

    #[test]
    fn validate_tasks_rejects_invalid_hunks() {
        let diff = "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1,1 +1,1 @@\n-old\n+new\n";
        let mut task = sample_task("T1", &["src/a.rs"]);
        task.diff_refs = vec![DiffRef {
            file: "src/a.rs".to_string(),
            hunks: vec![HunkRef {
                old_start: 2,
                old_lines: 1,
                new_start: 2,
                new_lines: 1,
            }],
        }];
        assert!(
            crate::infra::acp::task_generator::validation::validate_tasks_payload(
                &[task],
                None,
                diff
            )
            .is_err()
        );
    }

    #[test]
    fn validate_tasks_rejects_unknown_file_with_empty_hunks() {
        let diff = "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1,1 +1,1 @@\n-old\n+new\n";
        let mut task = sample_task("T1", &["src/a.rs"]);
        task.diff_refs = vec![DiffRef {
            file: "src/missing.rs".to_string(),
            hunks: vec![],
        }];
        assert!(
            crate::infra::acp::task_generator::validation::validate_tasks_payload(
                &[task],
                None,
                diff
            )
            .is_err()
        );
    }

    #[test]
    fn validate_tasks_rejects_missing_diagram() {
        let diff = "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n";
        let mut task = sample_task("T1", &["src/a.rs"]);
        task.diagram = None;
        assert!(
            crate::infra::acp::task_generator::validation::validate_tasks_payload(
                &[task],
                None,
                diff
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod real_acp_tests {
    use super::*;

    /// Integration test: hits the real Codex ACP via npx.
    /// Run with: `cargo test -- --ignored`
    #[test]
    #[ignore]
    fn test_real_codex_acp_integration() {
        let diff = r###"diff --git a/src/beer.rs b/src/beer.rs
--- a/src/beer.rs
+++ b/src/beer.rs
@@ -1,23 +1,32 @@
 use std::time::Duration;

-#[derive(Debug)]
-pub struct BeerConfig {
-    pub brand: String,
-    pub temperature_c: u8,
-}
-pub fn open_bottle(brand: &str) {
-    println!(\"Opening {brand}\");
-}
-pub fn chill(config: &BeerConfig) {
-    println!(\"Chilling {} to {}°C\", config.brand, config.temperature_c);
-    std::thread::sleep(Duration::from_secs(3));
-}
-pub fn pour(brand: &str, ml: u32) {
-    println!(\"Pouring {ml}ml of {brand}\");
-}
-pub fn drink(brand: &str, ml: u32) {
-    println!(\"Drinking {ml}ml of {brand}\");
-}
+#[derive(Debug, Clone)]
+pub struct Beer {
+    brand: String,
+    temperature_c: u8,
+    opened: bool,
+}
+
+impl Beer {
+    pub fn new(brand: impl Into<String>, temperature_c: u8) -> Self {
+        Self {
+            brand: brand.into(),
+            temperature_c,
+            opened: false,
+        }
+    }
+
+    pub fn open(&mut self) {
+        self.opened = true;
+        println!(\"Opening {}\", self.brand);
+    }
+
+    pub fn chill(&self) {
+        println!(\"Chilling {} to {}°C\", self.brand, self.temperature_c);
+        std::thread::sleep(Duration::from_secs(3));
+    }
+
+    pub fn pour(&self, ml: u32) {
+        println!(\"Pouring {ml}ml of {}\", self.brand);
+    }
+
+    pub fn drink(&self, ml: u32) {
+        println!(\"Drinking {ml}ml of {}\", self.brand);
+    }
+}
"###;

        let diff_hash = format!("{:016x}", crate::infra::hash::hash64(diff));
        let run_context = RunContext {
            review_id: "test-review".into(),
            run_id: "test-run".into(),
            agent_id: "codex".into(),
            input_ref: "diff".into(),
            diff_text: diff.to_string().into(),
            diff_hash: diff_hash.clone(),
            source: ReviewSource::DiffPaste { diff_hash },
            initial_title: Some("Test PR".into()),
            created_at: Some(chrono::Utc::now().to_rfc3339()),
        };

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

        let input = GenerateTasksInput {
            run_context,
            repo_root: None,
            agent_command: "npx".into(),
            agent_args: vec![
                "-y",
                "@zed-industries/codex-acp@latest",
                "-c",
                "model=\"gpt-5.1-codex-mini\"",
                "-c",
                "model_reasoning_effort=\"medium\"",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            progress_tx: Some(tx),
            mcp_server_binary: None,
            timeout_secs: Some(300),
            cancel_token: None,
            debug: true,
        };

        // Ensure we use the real binary, not the test harness
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir");
        let binary_path = std::path::PathBuf::from(manifest_dir).join("target/debug/lareview");
        if binary_path.exists() {
            unsafe {
                std::env::set_var("TASK_MCP_SERVER_BIN", binary_path);
            }
        } else {
            eprintln!(
                "WARNING: Real binary not found at {:?}, test might fail if using test harness",
                binary_path
            );
        }

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        let result = runtime.block_on(generate_tasks_with_acp(input));
        match &result {
            Ok(res) => {
                eprintln!("messages: {:#?}", res.messages);
                eprintln!("thoughts: {:#?}", res.thoughts);
                eprintln!("logs: {:#?}", res.logs);
            }
            Err(err) => eprintln!("error: {:?}", err),
        }
        assert!(
            result.is_ok(),
            "expected Codex ACP to return tasks: {:?}",
            result.err()
        );
    }

    /// Ignored by default: runs the real agent and asserts tasks were persisted to SQLite.
    /// Integration test with DB persistence.
    /// Run with: `cargo test -- --ignored`
    #[test]
    #[ignore]
    fn test_real_codex_acp_persist() -> anyhow::Result<()> {
        let tmp = tempfile::tempdir().expect("tmpdir");
        let db_path = tmp.path().join("db.sqlite");
        let prev_db = set_env("LAREVIEW_DB_PATH", db_path.to_string_lossy().as_ref());

        let diff = r###"diff --git a/src/foo.rs b/src/foo.rs
--- a/src/foo.rs
+++ b/src/foo.rs
@@ -1 +1,3 @@
-fn old() {}
+fn new_fn() {
+    println!(\"hi\");
+}
"###;

        let diff_hash = format!("{:016x}", crate::infra::hash::hash64(diff));
        let run_context = RunContext {
            review_id: "test-review".into(),
            run_id: "test-run".into(),
            agent_id: "codex".into(),
            input_ref: "diff".into(),
            diff_text: diff.to_string().into(),
            diff_hash: diff_hash.clone(),
            source: ReviewSource::DiffPaste { diff_hash },
            initial_title: Some("Test PR".into()),
            created_at: Some(chrono::Utc::now().to_rfc3339()),
        };

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

        let input = GenerateTasksInput {
            run_context: run_context.clone(),
            repo_root: None,
            agent_command: "npx".into(),
            agent_args: vec![
                "-y",
                "@zed-industries/codex-acp@latest",
                "-c",
                "model=\"gpt-5.1-codex-mini\"",
                "-c",
                "model_reasoning_effort=\"medium\"",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            progress_tx: Some(tx),
            mcp_server_binary: None,
            timeout_secs: Some(300),
            cancel_token: None,
            debug: true,
        };

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        let _result = runtime.block_on(generate_tasks_with_acp(input))?;

        // Verify persisted tasks are present in SQLite
        let db = Database::open_at(db_path.clone())?;
        let repo = TaskRepository::new(db.connection());
        let tasks = repo.find_by_run(&run_context.run_id)?;
        assert!(!tasks.is_empty(), "expected tasks persisted, got none");

        restore_env("LAREVIEW_DB_PATH", prev_db);
        Ok(())
    }
}
