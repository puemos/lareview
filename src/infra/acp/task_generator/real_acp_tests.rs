use super::*;
use crate::domain::ReviewSource;
use crate::infra::db::{Database, TaskRepository};

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

/// Integration test: hits the real Codex ACP via npx.
/// Run with: `cargo test -- --ignored`
#[test]
#[ignore]
fn test_real_codex_acp_integration() {
    let diff = r#"diff --git a/src/beer.rs b/src/beer.rs
--- a/src/beer.rs
+++ b/src/beer.rs
@@ -1,23 +1,32 @@
 use std::time::Duration;

-#[derive(Debug)]
-pub struct BeerConfig {
-    pub brand: String,
-    pub temperature_c: u8,
-}
-
-pub fn open_bottle(brand: &str) {
-    println!("Opening {brand}");
-}
-
-pub fn chill(config: &BeerConfig) {
-    println!("Chilling {} to {}°C", config.brand, config.temperature_c);
-    std::thread::sleep(Duration::from_secs(3));
-}
-
-pub fn pour(brand: &str, ml: u32) {
-    println!("Pouring {ml}ml of {brand}");
-}
-
-pub fn drink(brand: &str, ml: u32) {
-    println!("Drinking {ml}ml of {brand}");
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
+        println!("Opening {}", self.brand);
+    }
+
+    pub fn chill(&self) {
+        println!("Chilling {} to {}°C", self.brand, self.temperature_c);
+        std::thread::sleep(Duration::from_secs(3));
+    }
+
+    pub fn pour(&self, ml: u32) {
+        println!("Pouring {ml}ml of {}", self.brand);
+    }
+
+    pub fn drink(&self, ml: u32) {
+        println!("Drinking {ml}ml of {}", self.brand);
+    }
+}
"#;

    let diff_hash = format!("{:016x}", crate::infra::hash::hash64(diff));
    let run_context = crate::infra::acp::RunContext {
        review_id: "test-review".into(),
        run_id: "test-run".into(),
        agent_id: "codex".into(),
        input_ref: "diff".into(),
        diff_text: diff.to_string(),
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
        Err(err) => eprintln!("error: {err:?}"),
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

    let diff = r#"diff --git a/src/foo.rs b/src/foo.rs
--- a/src/foo.rs
+++ b/src/foo.rs
@@ -1 +1,3 @@
-fn old() {}
+fn new_fn() {
+    println!("hi");
+}
"#;

    let diff_hash = format!("{:016x}", crate::infra::hash::hash64(diff));
    let run_context = crate::infra::acp::RunContext {
        review_id: "test-review".into(),
        run_id: "test-run".into(),
        agent_id: "codex".into(),
        input_ref: "diff".into(),
        diff_text: diff.to_string(),
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
        debug: true,
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    let result = runtime.block_on(generate_tasks_with_acp(input))?;

    // Verify persisted tasks are present in SQLite
    let db = Database::open_at(db_path.clone())?;
    let repo = TaskRepository::new(db.connection());
    let tasks = repo.find_by_run(&run_context.run_id)?;
    assert!(
        !tasks.is_empty(),
        "expected tasks persisted, got none; logs: {:?}",
        result.logs
    );

    restore_env("LAREVIEW_DB_PATH", prev_db);
    Ok(())
}
