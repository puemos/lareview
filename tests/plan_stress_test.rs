use lareview::domain::ReviewSource;
use lareview::infra::acp::{
    GenerateTasksInput, ProgressEvent, RunContext, generate_tasks_with_acp,
};
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_plan_stress_with_fake_agent() {
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

    // Fallback for different build layouts
    if !agent_path.exists() {
        // Try to find it in the examples directory if it's already built by cargo
        agent_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/debug/examples/fake_acp_agent");
    }

    let agent_path_str = agent_path.to_str().unwrap();
    eprintln!("DEBUG: agent_path={}", agent_path_str);
    assert!(
        agent_path.exists(),
        "Agent binary NOT FOUND at {}",
        agent_path_str
    );

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

    let (tx, mut rx) = mpsc::unbounded_channel();

    let input = GenerateTasksInput {
        run_context,
        rules: Vec::new(),
        repo_root: None,
        cleanup_path: None,
        agent_command: agent_path_str.to_string(),
        agent_args: Vec::new(),
        progress_tx: Some(tx),
        mcp_server_binary: None,
        timeout_secs: Some(10),
        cancel_token: None,
        debug: true,
    };

    // Set environment variable for fake agent
    unsafe {
        std::env::set_var("PLAN_STRESS", "1");
    }

    let handle = tokio::spawn(async move { generate_tasks_with_acp(input).await });

    let mut plan_events = Vec::new();
    while let Some(event) = rx.recv().await {
        eprintln!("DEBUG: received event: {:?}", event);
        if let ProgressEvent::Plan(plan) = event {
            plan_events.push(plan);
        }
    }

    let result = handle.await.unwrap();
    eprintln!("DEBUG: task generation result: {:?}", result);

    // We expect 2 plan events from the fake agent in PLAN_STRESS mode
    assert!(
        plan_events.len() >= 2,
        "Expected at least 2 plan events, got {}",
        plan_events.len()
    );

    // First plan should have 1 entry
    assert_eq!(plan_events[0].entries.len(), 1);
    assert_eq!(plan_events[0].entries[0].content, "Task 1");

    // Second plan should have 2 entries
    assert_eq!(plan_events[1].entries.len(), 2);
    assert_eq!(plan_events[1].entries[0].content, "Task 1");
    assert_eq!(plan_events[1].entries[1].content, "Task 2");
}
