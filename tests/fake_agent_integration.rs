use lareview::domain::ReviewSource;
use lareview::infra::acp::RunContext;
use lareview::infra::acp::{GenerateTasksInput, generate_tasks_with_acp};
use std::sync::Arc;

#[tokio::test]
async fn test_generate_tasks_with_fake_agent_finalize_only() {
    let mut agent_path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/fake_acp_agent");
    if !agent_path.exists() {
        agent_path = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples/fake_acp_agent");
    }
    let agent_path = agent_path.to_str().unwrap();
    let diff_text = "diff --git a/src/a.rs b/src/a.rs\n\
index 0000000..1111111 100644\n\
--- a/src/a.rs\n\
+++ b/src/a.rs\n\
@@ -0,0 +1 @@\n\
+line\n";
    let run_context = RunContext {
        review_id: "review".to_string(),
        run_id: "run".to_string(),
        agent_id: "fake".to_string(),
        input_ref: "diff".to_string(),
        diff_text: Arc::from(diff_text),
        diff_hash: "hash".to_string(),
        source: ReviewSource::DiffPaste {
            diff_hash: "hash".to_string(),
        },
        initial_title: None,
        created_at: None,
    };

    let input = GenerateTasksInput {
        run_context,
        rules: Vec::new(),
        repo_root: None,
        cleanup_path: None,
        agent_command: agent_path.to_string(),
        agent_args: Vec::new(),
        progress_tx: None,
        mcp_server_binary: None,
        timeout_secs: Some(10),
        cancel_token: None,
        debug: true,
    };

    let result = generate_tasks_with_acp(input).await;
    let error = result.unwrap_err().to_string();
    assert!(error.contains("Tasks do not cover all changed files"));
    assert!(error.contains("src/a.rs"));
}
