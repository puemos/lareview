use crate::domain::{PullRequest, ReviewTask};

fn sample_pr() -> PullRequest {
    PullRequest {
        id: "pr-1".into(),
        title: "Test".into(),
        description: None,
        repo: "example/repo".into(),
        author: "tester".into(),
        branch: "main".into(),
        created_at: String::new(),
    }
}

fn sample_task(id: &str, files: &[&str]) -> ReviewTask {
    ReviewTask {
        id: id.into(),
        pr_id: "pr-1".into(),
        title: id.into(),
        description: String::new(),
        files: files.iter().map(|f| f.to_string()).collect(),
        stats: crate::domain::TaskStats {
            additions: 0,
            deletions: 0,
            risk: crate::domain::RiskLevel::Low,
            tags: vec![],
        },
        diffs: vec![],
        insight: None,
        diagram: None,
        ai_generated: true,
        status: crate::domain::TaskStatus::Pending,
        sub_flow: None,
    }
}

#[test]
fn prompt_renders_no_repo_access_block() {
    let pr = sample_pr();
    let diff = "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n";
    let prompt = super::prompt::build_prompt(&pr, diff, None);
    assert!(prompt.contains("You do NOT have repository access."));
    assert!(prompt.contains("Do NOT call any tools except `return_tasks`."));
    assert!(!prompt.contains("You have READ-ONLY access"));
}

#[test]
fn prompt_renders_repo_access_block() {
    let pr = sample_pr();
    let diff = "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n";
    let root = std::path::PathBuf::from("/tmp/repo-root");
    let prompt = super::prompt::build_prompt(&pr, diff, Some(&root));
    assert!(prompt.contains("You have READ-ONLY access"));
    assert!(prompt.contains(&root.display().to_string()));
    assert!(prompt.contains("Allowed tools:"));
    assert!(!prompt.contains("You do NOT have repository access."));
}

#[test]
fn capabilities_disable_tools_without_repo() {
    let caps = super::prompt::build_client_capabilities(false);
    assert!(!caps.terminal);
    assert!(!caps.fs.read_text_file);
    assert!(!caps.fs.write_text_file);
}

#[test]
fn capabilities_readonly_fs_with_repo() {
    let caps = super::prompt::build_client_capabilities(true);
    assert!(!caps.terminal);
    assert!(caps.fs.read_text_file);
    assert!(!caps.fs.write_text_file);
}

#[tokio::test(flavor = "current_thread")]
async fn permission_cancelled_without_repo_access() {
    let client = super::client::LaReviewClient::new(None, "pr-1", None);
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
        <super::client::LaReviewClient as agent_client_protocol::Client>::request_permission(
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
async fn permission_allows_return_tasks_even_without_repo_access() {
    let client = super::client::LaReviewClient::new(None, "pr-1", None);
    let fields = agent_client_protocol::ToolCallUpdateFields::new()
        .kind(agent_client_protocol::ToolKind::Other)
        .title("return_tasks")
        .raw_input(serde_json::json!({ "tasks": [] }));
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
        <super::client::LaReviewClient as agent_client_protocol::Client>::request_permission(
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
async fn permission_allows_tasks_payload_embedded_in_title() {
    let client = super::client::LaReviewClient::new(None, "pr-1", None);
    let fields = agent_client_protocol::ToolCallUpdateFields::new()
        .kind(agent_client_protocol::ToolKind::Other)
        .title(r#"{"tasks":[{"id":"T1","title":"Example","description":"","files":[],"stats":{"additions":0,"deletions":0,"risk":"LOW","tags":[]},"patches":[]}]}"#);
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
        <super::client::LaReviewClient as agent_client_protocol::Client>::request_permission(
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

    let client = super::client::LaReviewClient::new(None, "pr-1", Some(root.path().to_path_buf()));
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
        <super::client::LaReviewClient as agent_client_protocol::Client>::request_permission(
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

    let client = super::client::LaReviewClient::new(None, "pr-1", Some(root.path().to_path_buf()));
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
        <super::client::LaReviewClient as agent_client_protocol::Client>::request_permission(
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
    let client = super::client::LaReviewClient::new(None, "pr-1", Some(root.path().to_path_buf()));
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
        <super::client::LaReviewClient as agent_client_protocol::Client>::request_permission(
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
    assert!(super::validation::validate_tasks_payload(&tasks, Some(&raw), diff).is_ok());

    let missing = vec![sample_task("T1", &[]), sample_task("T2", &[])];
    assert!(super::validation::validate_tasks_payload(&missing, Some(&raw), diff).is_err());
}
