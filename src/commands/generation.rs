use crate::application::review::rules::resolve_rules;
use crate::domain::{ResolvedRule, Review, ReviewRun, ReviewRunStatus, ReviewSource, ReviewStatus};
use crate::infra::acp::{
    AgentConfigSelection, GenerateTasksInput, ProgressEvent, RunContext, generate_tasks_with_acp,
    list_agent_candidates,
};
use crate::infra::diff::index::DiffIndex;
use crate::infra::hash::hash_diff;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{State, ipc::Channel};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum ProgressEventPayload {
    Log(String),
    // Delta streaming for text
    MessageDelta {
        id: String,
        delta: String,
    },
    ThoughtDelta {
        id: String,
        delta: String,
    },
    // Two-phase tool calls
    ToolCallStarted {
        tool_call_id: String,
        title: String,
        kind: String,
    },
    ToolCallComplete {
        tool_call_id: String,
        status: String,
        title: String,
        raw_input: Option<serde_json::Value>,
        raw_output: Option<serde_json::Value>,
    },
    // Other events
    Plan(FrontendPlan),
    TaskStarted {
        task_id: String,
        title: String,
    },
    TaskCompleted {
        task_id: String,
    },
    Completed {
        task_count: usize,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendPlan {
    pub entries: Vec<FrontendPlanEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendPlanEntry {
    pub content: String,
    pub priority: String,
    pub status: String,
}

struct SnapshotCleanupGuard {
    path: Option<std::path::PathBuf>,
}

impl SnapshotCleanupGuard {
    fn new(path: Option<std::path::PathBuf>) -> Self {
        Self { path }
    }
}

impl Drop for SnapshotCleanupGuard {
    fn drop(&mut self) {
        if let Some(path) = &self.path
            && path.exists()
        {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

async fn cleanup_snapshot(path: &std::path::Path) {
    let mut retries = 5;
    let mut delay = std::time::Duration::from_millis(200);

    loop {
        if !path.exists() {
            break;
        }

        if let Err(e) = std::fs::remove_dir_all(path) {
            log::warn!("Failed to cleanup snapshot {}: {}", path.display(), e);
        }

        if !path.exists() {
            break;
        }

        retries -= 1;
        if retries == 0 {
            break;
        }

        tokio::time::sleep(delay).await;
        delay *= 2;
    }
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn generate_review(
    state: State<'_, AppState>,
    diff_text: String,
    agent_id: String,
    run_id: Option<String>,
    repo_id: Option<String>,
    source: Option<ReviewSource>,
    use_snapshot: bool,
    agent_config: Option<Vec<AgentConfigSelection>>,
    on_progress: Channel<ProgressEventPayload>,
) -> Result<ReviewGenerationResult, String> {
    generate_review_inner(
        state.inner(),
        diff_text,
        agent_id,
        run_id,
        repo_id,
        source,
        use_snapshot,
        agent_config,
        on_progress,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn generate_review_inner(
    state: &AppState,
    diff_text: String,
    agent_id: String,
    run_id: Option<String>,
    repo_id: Option<String>,
    source: Option<ReviewSource>,
    use_snapshot: bool,
    agent_config: Option<Vec<AgentConfigSelection>>,
    on_progress: Channel<ProgressEventPayload>,
) -> Result<ReviewGenerationResult, String> {
    let diff_hash = hash_diff(&diff_text);
    let review_id = Uuid::new_v4().to_string();
    let run_id = run_id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let source = source.unwrap_or_else(|| ReviewSource::DiffPaste {
        diff_hash: diff_hash.clone(),
    });

    // Create snapshot if requested and applicable
    let snapshot_path = if use_snapshot {
        let repo_id_ref = &repo_id;
        let head_sha = match &source {
            ReviewSource::GitHubPr {
                head_sha: Some(head_sha),
                ..
            } => Some(head_sha.as_str()),
            ReviewSource::GitLabMr {
                head_sha: Some(head_sha),
                ..
            } => Some(head_sha.as_str()),
            _ => None,
        };

        if let (Some(rid), Some(head_sha)) = (repo_id_ref, head_sha) {
            let repos = {
                let db = state.db.lock().map_err(|e| e.to_string())?;
                db.get_linked_repos().map_err(|e| e.to_string())?
            };
            if let Some(repo) = repos.iter().find(|r| r.id == *rid) {
                let manager = crate::infra::vcs::snapshot::SnapshotManager::new(
                    std::path::PathBuf::from(&repo.path),
                );

                // Notify via progress channel
                let _ = on_progress.send(ProgressEventPayload::Log(format!(
                    "Creating snapshot for {} at {}...",
                    repo.name,
                    &head_sha[..7]
                )));

                let snapshot_path = manager
                    .create(&run_id, head_sha)
                    .await
                    .map_err(|e| e.to_string())?;

                let _ = on_progress.send(ProgressEventPayload::Log(format!(
                    "Snapshot ready at {}",
                    snapshot_path.display()
                )));

                Some(snapshot_path)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let _snapshot_guard = SnapshotCleanupGuard::new(snapshot_path.clone());

    let now = chrono::Utc::now().to_rfc3339();

    let run = ReviewRun {
        id: run_id.clone(),
        review_id: review_id.clone(),
        agent_id: agent_id.clone(),
        input_ref: format!("diff-{}", &diff_hash[..8]),
        diff_text: Arc::from(diff_text.as_str()),
        diff_hash: diff_hash.clone(),
        status: ReviewRunStatus::Running,
        created_at: now.clone(),
    };

    let initial_title = match &source {
        ReviewSource::GitHubPr { repo, number, .. } => {
            format!("PR {}#{}", repo, number)
        }
        ReviewSource::GitLabMr {
            project_path,
            number,
            ..
        } => format!("MR {}!{}", project_path, number),
        _ => "AI Review".to_string(),
    };

    let review = Review {
        id: review_id.clone(),
        title: initial_title,
        summary: None,
        source: source.clone(),
        active_run_id: Some(run_id.clone()),
        status: ReviewStatus::Todo,
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    let (candidate_label, command, candidate_args) = {
        let candidates = list_agent_candidates();
        let agent_candidate = candidates
            .iter()
            .find(|c| c.id == agent_id)
            .or_else(|| candidates.iter().find(|c| c.id == "default"))
            .ok_or_else(|| "No agent found".to_string())?;

        let candidate_label = agent_candidate.label.clone();
        let command = agent_candidate.command.clone().ok_or_else(|| {
            format!(
                "Agent '{}' is not available. Please configure the agent path in settings.",
                agent_id
            )
        })?;

        let candidate_args = agent_candidate.args.clone();

        (candidate_label, command, candidate_args)
    };

    let run_context = RunContext {
        review_id: review_id.clone(),
        run_id: run_id.clone(),
        agent_id: agent_id.clone(),
        input_ref: run.input_ref.clone(),
        diff_text: Arc::from(diff_text.as_str()),
        diff_hash,
        source,
        initial_title: None,
        created_at: Some(now),
    };

    let (mcp_tx, mut mcp_rx) = mpsc::unbounded_channel::<ProgressEvent>();

    let on_progress_clone = on_progress.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = mcp_rx.recv().await {
            let payload = match event {
                ProgressEvent::LocalLog(msg) => ProgressEventPayload::Log(msg),
                ProgressEvent::MessageDelta { id, delta } => {
                    ProgressEventPayload::MessageDelta { id, delta }
                }
                ProgressEvent::ThoughtDelta { id, delta } => {
                    ProgressEventPayload::ThoughtDelta { id, delta }
                }
                ProgressEvent::ToolCallStarted {
                    tool_call_id,
                    title,
                    kind,
                } => ProgressEventPayload::ToolCallStarted {
                    tool_call_id,
                    title,
                    kind,
                },
                ProgressEvent::ToolCallComplete {
                    tool_call_id,
                    status,
                    title,
                    raw_input,
                    raw_output,
                } => ProgressEventPayload::ToolCallComplete {
                    tool_call_id,
                    status,
                    title,
                    raw_input,
                    raw_output,
                },
                ProgressEvent::Plan(plan) => {
                    let plan_value = serde_json::to_value(&plan).unwrap_or_default();
                    let entries = if let Some(entries_val) =
                        plan_value.get("entries").and_then(|v| v.as_array())
                    {
                        entries_val
                            .iter()
                            .map(|e| {
                                let content = e
                                    .get("content")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let priority = e
                                    .get("priority")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Medium")
                                    .to_string();
                                let status = e
                                    .get("status")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Pending")
                                    .to_string();
                                FrontendPlanEntry {
                                    content,
                                    priority,
                                    status,
                                }
                            })
                            .collect()
                    } else {
                        Vec::new()
                    };
                    ProgressEventPayload::Plan(FrontendPlan { entries })
                }
                ProgressEvent::TaskStarted(id, title) => {
                    ProgressEventPayload::TaskStarted { task_id: id, title }
                }
                ProgressEvent::TaskAdded(id) => ProgressEventPayload::TaskCompleted { task_id: id },
                ProgressEvent::FeedbackAdded => {
                    ProgressEventPayload::Log("Feedback added".to_string())
                }
                ProgressEvent::MetadataUpdated => {
                    ProgressEventPayload::Log("Metadata updated".to_string())
                }
                ProgressEvent::Finalized => ProgressEventPayload::Completed { task_count: 0 },
            };
            if let Err(e) = on_progress_clone.send(payload) {
                log::error!("Failed to send progress to channel: {:?}", e);
                break;
            }
        }
    });

    let _ = on_progress.send(ProgressEventPayload::Log(format!(
        "Starting review generation with {}...",
        candidate_label
    )));

    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.save_review(&review).map_err(|e| e.to_string())?;
        db.save_run(&run).map_err(|e| e.to_string())?;
    }

    let cancel_token = CancellationToken::new();
    {
        let mut active = state.active_runs.lock().unwrap();
        active.insert(run_id.clone(), cancel_token.clone());
    }

    let repo_id = repo_id.and_then(|id| {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    let diff_paths = DiffIndex::new(&diff_text)
        .map(|index| index.get_all_file_paths())
        .unwrap_or_default();

    let rules: Vec<ResolvedRule> = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let all_rules = db.rule_repo().list_enabled().map_err(|e| e.to_string())?;
        resolve_rules(&all_rules, repo_id.as_deref(), &diff_paths)
    };

    // Use snapshot path as repo_root if provided for agent access
    let repo_root = snapshot_path.clone();

    let result = generate_tasks_with_acp(GenerateTasksInput {
        run_context,
        rules,
        repo_root,
        cleanup_path: snapshot_path.clone(),
        agent_command: command,
        agent_args: candidate_args,
        agent_config: agent_config.unwrap_or_default(),
        progress_tx: Some(mcp_tx),
        mcp_server_binary: None,
        timeout_secs: Some(
            crate::infra::app_config::load_config()
                .review_timeout_secs
                .unwrap_or(1000),
        ),
        cancel_token: Some(cancel_token),
        debug: std::env::var("RUST_LOG")
            .map(|v| v.contains("acp"))
            .unwrap_or(false),
    })
    .await;

    // Cleanup: remove token from active_runs
    {
        let mut active = state.active_runs.lock().unwrap();
        active.remove(&run_id);
    }

    if let Some(snapshot_path) = snapshot_path.as_ref() {
        cleanup_snapshot(snapshot_path).await;
    }

    match result {
        Ok(_) => {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            let tasks_result = db.get_tasks_by_run(&run_id);
            let task_count = tasks_result.map(|t| t.len()).unwrap_or(0);

            if let Err(err) = db
                .run_repo()
                .update_status(&run_id, ReviewRunStatus::Completed)
            {
                log::error!(
                    "Failed to update run status to completed for {}: {}",
                    run_id,
                    err
                );
            }

            let _ = on_progress.send(ProgressEventPayload::Completed { task_count });
        }
        Err(e) => {
            log::error!("Task generation failed: {:?}", e);
            let _ = on_progress.send(ProgressEventPayload::Error {
                message: format!("Generation failed: {}", e),
            });
            let db = state.db.lock().map_err(|e| e.to_string())?;
            let is_cancelled = e.to_string().contains("cancelled by user");
            let status = if is_cancelled {
                ReviewRunStatus::Cancelled
            } else {
                ReviewRunStatus::Failed
            };
            if let Err(err) = db.run_repo().update_status(&run_id, status) {
                log::error!(
                    "Failed to update run status for {} to {:?}: {}",
                    run_id,
                    status,
                    err
                );
            }

            if is_cancelled {
                let _ = db.review_repo().delete(&review_id);
                return Err("cancelled by user".to_string());
            }

            return Err(e.to_string());
        }
    }

    Ok(ReviewGenerationResult {
        task_count: 0,
        review_id,
        run_id: Some(run_id),
    })
}

#[tauri::command]
pub async fn stop_generation(state: State<'_, AppState>, run_id: String) -> Result<(), String> {
    let token = {
        let active = state.active_runs.lock().unwrap();
        active.get(&run_id).cloned()
    };

    if let Some(token) = token {
        token.cancel();
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewGenerationResult {
    pub task_count: usize,
    pub review_id: String,
    pub run_id: Option<String>,
}
