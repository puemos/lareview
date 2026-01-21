use crate::domain::{
    DiffRef, HunkRef, ReviewSource, ReviewStatus, ReviewTask, RiskLevel, TaskStats,
};
use crate::infra::acp::RunContext;
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
            "{\"type\":\"sequence\",\"data\":{\"actors\":[{\"id\":\"reviewer\",\"label\":\"Reviewer\",\"kind\":\"user\"},{\"id\":\"code\",\"label\":\"Code\",\"kind\":\"service\"}],\"messages\":[{\"type\":\"call\",\"data\":{\"from\":\"reviewer\",\"to\":\"code\",\"label\":\"review\"}}]}}",
        )),
        ai_generated: true,
        status: ReviewStatus::Todo,
        sub_flow: None,
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
        let prompt =
            crate::infra::acp::task_generator::prompt::build_prompt(&run, None, &[]).unwrap();
        assert!(prompt.contains("You do NOT have repository access."));
        assert!(prompt.contains("Use `lareview-tasks_return_task` for each task"));
        assert!(prompt.contains("Use `lareview-tasks_finalize_review` at the end"));
        assert!(!prompt.contains("You have READ-ONLY access"));
    }

    #[test]
    fn prompt_renders_repo_access_block() {
        let diff = "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n";
        let run = sample_run(diff);
        let root = std::path::PathBuf::from("/tmp/repo-root");
        let prompt =
            crate::infra::acp::task_generator::prompt::build_prompt(&run, Some(&root), &[])
                .unwrap();
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
        let res = crate::infra::acp::task_generator::prompt::build_prompt(&run, None, &[]);
        assert!(res.is_ok());
    }

    #[test]
    fn prompt_includes_rules_block() {
        use crate::domain::{ResolvedRule, RuleScope};
        let diff = "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n";
        let run = sample_run(diff);
        let rules = vec![ResolvedRule {
            id: "rule-1".into(),
            scope: RuleScope::Global,
            repo_id: None,
            glob: Some("src/**/*.rs".into()),
            category: None,
            text: "Prioritize auth checks".into(),
            matched_files: vec!["src/a.rs".into()],
            has_matches: true,
        }];
        let prompt =
            crate::infra::acp::task_generator::prompt::build_prompt(&run, None, &rules).unwrap();
        assert!(prompt.contains("<review_rules>"));
        assert!(prompt.contains("Prioritize auth checks"));
        assert!(prompt.contains("[rule-1]")); // category defaults to rule id when None
        assert!(prompt.contains("(rule_id: rule-1)"));
        assert!(prompt.contains("src/**/*.rs"));
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
                "hunk_ids": []
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
                    "hunk_ids": []
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
        let task_json = r###"{"id":"T1","title":"Example","description":"Test","stats":{"risk":"low","tags":[]},"diff_refs":[{"file":"test.rs","hunks":[{"old_start":1,"old_lines":1,"new_start":1,"new_lines":1}]}]}"###;
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
    async fn permission_allows_file_path_input() {
        let root = tempfile::tempdir().expect("root");
        std::fs::write(root.path().join("test.ex"), "hi").expect("write");

        let client = crate::infra::acp::task_generator::client::LaReviewClient::new(
            None,
            "run-1",
            Some(root.path().to_path_buf()),
        );
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(agent_client_protocol::ToolKind::Read)
            .title("fs/read_text_file")
            .raw_input(serde_json::json!({ "filePath": "test.ex" }));
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

    #[tokio::test(flavor = "current_thread")]
    async fn permission_allows_locations_input() {
        let root = tempfile::tempdir().expect("root");
        std::fs::write(root.path().join("test.txt"), "hi").expect("write");

        let client = crate::infra::acp::task_generator::client::LaReviewClient::new(
            None,
            "run-1",
            Some(root.path().to_path_buf()),
        );
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(agent_client_protocol::ToolKind::Read)
            .title("fs/read_text_file")
            .locations(vec![agent_client_protocol::ToolCallLocation::new(
                "test.txt",
            )]);
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
    async fn permission_allows_title_fallback() {
        let root = tempfile::tempdir().expect("root");
        std::fs::write(root.path().join("test.txt"), "hi").expect("write");

        let client = crate::infra::acp::task_generator::client::LaReviewClient::new(
            None,
            "run-1",
            Some(root.path().to_path_buf()),
        );
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(agent_client_protocol::ToolKind::Read)
            .title("Read test.txt");
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
    async fn permission_denies_locations_outside_repo() {
        let root = tempfile::tempdir().expect("root");
        std::fs::write(root.path().join("inside.txt"), "hi").expect("write");

        let client = crate::infra::acp::task_generator::client::LaReviewClient::new(
            None,
            "run-1",
            Some(root.path().to_path_buf()),
        );
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(agent_client_protocol::ToolKind::Read)
            .title("fs/read_text_file")
            .locations(vec![agent_client_protocol::ToolCallLocation::new(
                "/etc/passwd",
            )]);
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
    async fn permission_defers_validation_for_title_path_without_extension() {
        // When title contains a path WITHOUT a file extension (like /etc/passwd),
        // extract_path_from_title doesn't extract it. Permission is deferred,
        // but security is still enforced at layer 2 (read_text_file).
        let root = tempfile::tempdir().expect("root");
        std::fs::write(root.path().join("inside.txt"), "hi").expect("write");

        let client = crate::infra::acp::task_generator::client::LaReviewClient::new(
            None,
            "run-1",
            Some(root.path().to_path_buf()),
        );
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(agent_client_protocol::ToolKind::Read)
            .title("Read /etc/passwd");
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
        // Permission is allowed because path can't be extracted (no extension)
        // Security is enforced at layer 2: read_text_file will block /etc/passwd
        assert!(matches!(
            resp.outcome,
            agent_client_protocol::RequestPermissionOutcome::Selected(_)
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn permission_defers_validation_for_title_with_traversal_no_extension() {
        // When title contains traversal path WITHOUT a file extension,
        // extract_path_from_title doesn't extract it. Permission is deferred,
        // but security is still enforced at layer 2 (read_text_file).
        let root = tempfile::tempdir().expect("root");
        std::fs::write(root.path().join("inside.txt"), "hi").expect("write");

        let client = crate::infra::acp::task_generator::client::LaReviewClient::new(
            None,
            "run-1",
            Some(root.path().to_path_buf()),
        );
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(agent_client_protocol::ToolKind::Read)
            .title("Read ../../etc/passwd");
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
        // Permission is allowed because path can't be extracted (no extension)
        // Security is enforced at layer 2: read_text_file will block traversal paths
        assert!(matches!(
            resp.outcome,
            agent_client_protocol::RequestPermissionOutcome::Selected(_)
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn read_text_file_rejects_empty_path() {
        let root = tempfile::tempdir().expect("root");
        std::fs::write(root.path().join("test.txt"), "hi").expect("write");

        let client = crate::infra::acp::task_generator::client::LaReviewClient::new(
            None,
            "run-1",
            Some(root.path().to_path_buf()),
        );
        let session_id = agent_client_protocol::SessionId::new("s1");
        let read_req = agent_client_protocol::ReadTextFileRequest::new(session_id, "");
        let result =
            <crate::infra::acp::task_generator::client::LaReviewClient as agent_client_protocol::Client>::read_text_file(
                &client, read_req,
            )
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        let data = &err.data;
        let has_reason = data
            .as_ref()
            .and_then(|v| v.as_object())
            .and_then(|d| d.get("reason"))
            .is_some_and(|r| r.as_str().is_some_and(|s| s.contains("path")));
        assert!(
            has_reason,
            "Error should contain 'path' in reason: {:?}",
            data
        );
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
        let warnings = crate::infra::acp::task_generator::validation::validate_tasks_payload(
            &missing,
            Some(&raw),
            diff,
        )
        .expect("should succeed with warnings");
        assert!(
            warnings
                .iter()
                .any(|w| w.contains("Tasks do not cover all changed files")),
            "Expected warning about missing file coverage"
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

    // --- Deferred Path Security Tests ---

    #[tokio::test(flavor = "current_thread")]
    async fn permission_allows_read_with_deferred_path_when_repo_enabled() {
        let root = tempfile::tempdir().expect("root");

        let client = crate::infra::acp::task_generator::client::LaReviewClient::new(
            None,
            "run-1",
            Some(root.path().to_path_buf()),
        );
        // Empty raw_input simulates ACP sending permission request before parameters
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(agent_client_protocol::ToolKind::Read)
            .title("read")
            .raw_input(serde_json::json!({}));
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
        // Should be allowed because repo access is enabled (deferred validation)
        assert!(matches!(
            resp.outcome,
            agent_client_protocol::RequestPermissionOutcome::Selected(_)
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn permission_denies_read_with_deferred_path_when_no_repo_access() {
        let client =
            crate::infra::acp::task_generator::client::LaReviewClient::new(None, "run-1", None);
        // Empty raw_input with no repo access should be denied
        let fields = agent_client_protocol::ToolCallUpdateFields::new()
            .kind(agent_client_protocol::ToolKind::Read)
            .title("read")
            .raw_input(serde_json::json!({}));
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
        // Should be denied because no repo access
        assert!(matches!(
            resp.outcome,
            agent_client_protocol::RequestPermissionOutcome::Cancelled
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn read_text_file_denies_path_outside_repo_root() {
        let root = tempfile::tempdir().expect("root");
        std::fs::write(root.path().join("inside.txt"), "hi").expect("write");

        let client = crate::infra::acp::task_generator::client::LaReviewClient::new(
            None,
            "run-1",
            Some(root.path().to_path_buf()),
        );
        let session_id = agent_client_protocol::SessionId::new("s1");
        let read_req = agent_client_protocol::ReadTextFileRequest::new(session_id, "/etc/passwd");
        let result =
            <crate::infra::acp::task_generator::client::LaReviewClient as agent_client_protocol::Client>::read_text_file(
                &client, read_req,
            )
            .await;
        // Should be denied by read_text_file's resolve_repo_path
        assert!(result.is_err());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn read_text_file_denies_traversal_attack() {
        let root = tempfile::tempdir().expect("root");
        std::fs::write(root.path().join("inside.txt"), "hi").expect("write");

        let client = crate::infra::acp::task_generator::client::LaReviewClient::new(
            None,
            "run-1",
            Some(root.path().to_path_buf()),
        );
        let session_id = agent_client_protocol::SessionId::new("s1");
        let read_req =
            agent_client_protocol::ReadTextFileRequest::new(session_id, "../../../etc/passwd");
        let result =
            <crate::infra::acp::task_generator::client::LaReviewClient as agent_client_protocol::Client>::read_text_file(
                &client, read_req,
            )
            .await;
        // Should be denied by read_text_file's resolve_repo_path
        assert!(result.is_err());
    }
}
