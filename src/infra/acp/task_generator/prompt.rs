use crate::domain::PullRequest;
use crate::prompts;
use agent_client_protocol::{ClientCapabilities, FileSystemCapability, Meta};
use serde_json::json;
use std::path::PathBuf;

pub(super) fn build_prompt(
    pr: &PullRequest,
    diff_text: &str,
    repo_root: Option<&PathBuf>,
) -> String {
    let has_repo_access = repo_root.is_some();
    prompts::render(
        "generate_tasks",
        &json!({
            "id": pr.id,
            "title": pr.title,
            "repo": pr.repo,
            "author": pr.author,
            "branch": pr.branch,
            "diff": diff_text,
            "has_repo_access": has_repo_access,
            "repo_root": repo_root.map(|p| p.display().to_string()),
            "repo_access_note": if has_repo_access { "read-only" } else { "none" }
        }),
    )
    .expect("failed to render generate_tasks prompt")
}

pub(super) fn build_client_capabilities(has_repo_access: bool) -> ClientCapabilities {
    let fs_cap = if has_repo_access {
        FileSystemCapability::new()
            .read_text_file(true)
            .write_text_file(false)
    } else {
        FileSystemCapability::new()
            .read_text_file(false)
            .write_text_file(false)
    };

    ClientCapabilities::new()
        .fs(fs_cap)
        .terminal(false)
        .meta(Meta::from_iter([
            (
                "lareview-return-tasks".into(),
                serde_json::json!({
                    "type": "extension",
                    "method": "lareview/return_tasks",
                    "description": "Submit review tasks back to the client as structured data",
                    "params": {
                        "tasks": [{
                            "id": "string",
                            "title": "string",
                            "description": "string",
                            "files": ["string"],
                            "stats": {
                                "additions": "number",
                                "deletions": "number",
                                "risk": "LOW|MEDIUM|HIGH",
                                "tags": ["string"]
                            },
                            "patches": [{"file": "string", "hunk": "string"}]
                        }]
                    }
                }),
            ),
            (
                "lareview-return-plans".into(),
                serde_json::json!({
                    "type": "extension",
                    "method": "lareview/return_plans",
                    "description": "Submit review plans back to the client as structured data",
                    "params": {
                        "plans": [{
                            "entries": [{
                                "content": "string",
                                "priority": "LOW|MEDIUM|HIGH",
                                "status": "PENDING|IN_PROGRESS|COMPLETED",
                                "meta": "object"
                            }],
                            "meta": "object"
                        }]
                    }
                }),
            ),
        ]))
}
