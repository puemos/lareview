use crate::application::review::export::{ExportData, ExportOptions, ReviewExporter};
use crate::application::review::rules::resolve_rules;
use crate::domain::{
    Comment, Feedback, FeedbackAnchor, FeedbackImpact, FeedbackSide,
    LinkedRepo as DomainLinkedRepo, ResolvedRule, Review, ReviewRule, ReviewRun, ReviewRunStatus,
    ReviewSource, ReviewStatus, ReviewTask, RuleScope,
};
use crate::infra::acp::{
    AgentRegistry, GenerateTasksInput, ProgressEvent, RunContext, generate_tasks_with_acp,
    invalidate_agent_cache,
};
use crate::infra::diff::index::DiffIndex;
use crate::infra::hash::hash_diff;
use crate::infra::vcs::registry::VcsRegistry;
use crate::infra::vcs::traits::{
    FeedbackPushRequest, ReviewPushRequest, VcsCloneRequest, VcsStatus,
};
use crate::state::{AppState, PendingDiff};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliStatus {
    pub is_installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
}

#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub async fn get_cli_status() -> Result<CliStatus, String> {
    let path = which::which("lareview").ok();
    let is_installed = path.is_some();
    let path_str = path.as_ref().map(|p| p.to_string_lossy().to_string());

    let version = if is_installed {
        let output = std::process::Command::new("lareview")
            .arg("--version")
            .output()
            .ok();

        output.and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                // "lareview 0.0.18" -> "0.0.18"
                Some(s.replace("lareview ", ""))
            } else {
                None
            }
        })
    } else {
        None
    };

    Ok(CliStatus {
        is_installed,
        version,
        path: path_str,
    })
}

#[tauri::command]
pub async fn install_cli() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let target = std::path::Path::new("/usr/local/bin/lareview");

        // Ensure /usr/local/bin exists
        if let Some(parent) = target.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create /usr/local/bin: {}", e))?;
        }

        // Check if it already exists
        if target.exists() {
            // Check if it's already pointing to us
            if let Ok(existing) = std::fs::read_link(target)
                && existing == current_exe
            {
                return Ok(());
            }
            // Remove existing if it's different or not a symlink
            let _ = std::fs::remove_file(target);
        }

        // Try to symlink
        std::os::unix::fs::symlink(&current_exe, target).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                "Permission denied. Please ensure /usr/local/bin is writable or manually symlink lareview to /usr/local/bin.".to_string()
            } else {
                format!("Failed to create symlink: {}", e)
            }
        })?;

        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    Err("CLI installation is only supported on macOS".to_string())
}

#[tauri::command]
pub fn clear_pending_diff(state: State<'_, AppState>) -> Result<(), String> {
    let mut pending = state.pending_diff.lock().map_err(|e| e.to_string())?;
    *pending = None;
    Ok(())
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
        let registry = AgentRegistry::default();
        let agent_candidate = registry
            .get_agent_candidate(&agent_id)
            .or_else(|| registry.get_agent_candidate("default"))
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
        progress_tx: Some(mcp_tx),
        mcp_server_binary: None,
        timeout_secs: Some(1000),
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

#[tauri::command]
pub fn get_pending_reviews(state: State<'_, AppState>) -> Result<Vec<PendingReviewState>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let reviews = db.get_pending_reviews().map_err(|e| e.to_string())?;
    Ok(reviews)
}

#[tauri::command]
pub fn parse_diff(diff_text: String) -> Result<ParsedDiff, String> {
    let index = DiffIndex::new(&diff_text).map_err(|e| e.to_string())?;
    let manifest = index.generate_hunk_manifest_json();
    let file_paths = index.get_all_file_paths();

    let total_additions = diff_text
        .lines()
        .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
        .count();

    let total_deletions = diff_text
        .lines()
        .filter(|l| l.starts_with('-') && !l.starts_with("---"))
        .count();

    let files: Vec<ParsedDiffFile> = file_paths
        .iter()
        .map(|path| {
            let hunk_ids = index.get_hunk_ids_for_file(path);
            ParsedDiffFile {
                name: path.clone(),
                old_path: path.clone(),
                new_path: path.clone(),
                hunks: hunk_ids
                    .iter()
                    .filter_map(|hunk_id| {
                        index.get_hunk_coords(hunk_id).map(|coords| {
                            let content = index
                                .get_hunk_content_by_coords(
                                    path,
                                    coords.old_start,
                                    coords.new_start,
                                )
                                .unwrap_or_default();

                            ParsedHunk {
                                old_start: coords.old_start,
                                old_lines: coords.old_lines,
                                new_start: coords.new_start,
                                new_lines: coords.new_lines,
                                content,
                            }
                        })
                    })
                    .collect(),
            }
        })
        .collect();

    Ok(ParsedDiff {
        diff_text,
        total_additions,
        total_deletions,
        hunk_manifest: manifest,
        files,
        source: None,
        title: None,
    })
}

#[tauri::command]
pub fn get_file_content(
    repo_root: String,
    file_path: String,
    commit: String,
) -> Result<String, String> {
    use std::process::Command;
    let output = Command::new("git")
        .args(["show", &format!("{}:{}", commit, file_path)])
        .current_dir(&repo_root)
        .output()
        .map_err(|e| e.to_string())?;

    String::from_utf8(output.stdout).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_tasks(
    state: State<'_, AppState>,
    run_id: Option<String>,
) -> Result<Vec<ReviewTask>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tasks = if let Some(run_id) = run_id {
        db.get_tasks_by_run(&run_id).map_err(|e| e.to_string())?
    } else {
        db.get_all_tasks().map_err(|e| e.to_string())?
    };
    Ok(tasks)
}

#[tauri::command]
pub fn get_all_reviews(state: State<'_, AppState>) -> Result<Vec<ReviewState>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let reviews = db.get_all_reviews().map_err(|e| e.to_string())?;
    Ok(reviews)
}

#[tauri::command]
pub fn get_review_runs(
    state: State<'_, AppState>,
    review_id: String,
) -> Result<Vec<ReviewRunState>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let runs = db.get_review_runs(&review_id).map_err(|e| e.to_string())?;
    Ok(runs)
}

#[tauri::command]
pub fn get_linked_repos(state: State<'_, AppState>) -> Result<Vec<LinkedRepoState>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let repos = db.get_linked_repos().map_err(|e| e.to_string())?;
    Ok(repos)
}

#[tauri::command]
pub fn set_repo_snapshot_access(
    state: State<'_, AppState>,
    repo_id: String,
    allowed: bool,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.repo_repo()
        .update_snapshot_access(&repo_id, allowed)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn update_task_status(
    state: State<'_, AppState>,
    task_id: String,
    status: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let review_status = ReviewStatus::from_str(&status).unwrap_or(ReviewStatus::Todo);
    db.update_task_status(&task_id, review_status)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn save_feedback(
    state: State<'_, AppState>,
    feedback: FeedbackInput,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();

    let anchor = if let (Some(file_path), Some(line_number), Some(side)) =
        (feedback.file_path, feedback.line_number, feedback.side)
    {
        Some(FeedbackAnchor {
            file_path: Some(file_path),
            line_number: Some(line_number),
            side: if side == "old" {
                Some(FeedbackSide::Old)
            } else {
                Some(FeedbackSide::New)
            },
            hunk_ref: None,
            head_sha: None,
        })
    } else {
        None
    };

    let impact = FeedbackImpact::from_str(&feedback.impact).unwrap_or(FeedbackImpact::Nitpick);

    let feedback_domain = Feedback {
        id: id.clone(),
        review_id: feedback.review_id,
        task_id: feedback.task_id,
        rule_id: feedback.rule_id,
        finding_id: None,
        category: None,
        title: feedback.title,
        status: ReviewStatus::Todo,
        impact,
        confidence: 1.0, // User-created feedback is high confidence
        anchor,
        author: "user".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    db.save_feedback(&feedback_domain, &id)
        .map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn get_feedback_comments(
    state: State<'_, AppState>,
    feedback_id: String,
) -> Result<Vec<Comment>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let comments = db
        .get_comments_for_feedback(&feedback_id)
        .map_err(|e| e.to_string())?;
    Ok(comments)
}

#[tauri::command]
pub fn add_comment(
    state: State<'_, AppState>,
    feedback_id: String,
    body: String,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    let comment = Comment {
        id: id.clone(),
        feedback_id,
        author: "user".to_string(),
        body,
        parent_id: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    db.save_comment(&comment).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn update_feedback_status(
    state: State<'_, AppState>,
    feedback_id: String,
    status: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let review_status = ReviewStatus::from_str(&status).unwrap_or(ReviewStatus::Todo);

    // If status is being set to "ignored", record the rejection
    if review_status == ReviewStatus::Ignored {
        // Fetch feedback details for rejection tracking
        if let Ok(Some(feedback)) = db.feedback_repo().find_by_id(&feedback_id) {
            let rejection_repo = db.rejection_repo();

            // Only record if not already recorded
            if !rejection_repo
                .rejection_exists(&feedback_id)
                .unwrap_or(false)
            {
                // Extract agent_id from author (format: "agent:agent_id")
                let agent_id = feedback
                    .author
                    .strip_prefix("agent:")
                    .unwrap_or(&feedback.author)
                    .to_string();

                // Extract file extension from anchor if available
                let file_extension = feedback
                    .anchor
                    .as_ref()
                    .and_then(|a| a.file_path.as_ref())
                    .and_then(|p| {
                        std::path::Path::new(p)
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(|s| s.to_string())
                    });

                let _ = rejection_repo.record_rejection(
                    &feedback_id,
                    &feedback.review_id,
                    feedback.rule_id.as_deref(),
                    &agent_id,
                    &feedback.impact.to_string(),
                    feedback.confidence,
                    file_extension.as_deref(),
                    &feedback.title,
                );
            }
        }
    }

    db.update_feedback_status(&feedback_id, review_status)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn update_feedback_impact(
    state: State<'_, AppState>,
    feedback_id: String,
    impact: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let feedback_impact = FeedbackImpact::from_str(&impact).unwrap_or(FeedbackImpact::Nitpick);
    db.update_feedback_impact(&feedback_id, feedback_impact)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_feedback(state: State<'_, AppState>, feedback_id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_feedback(&feedback_id)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_review(state: State<'_, AppState>, review_id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.review_repo()
        .delete(&review_id)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_feedback_by_review(
    state: State<'_, AppState>,
    review_id: String,
) -> Result<Vec<Feedback>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let feedbacks = db
        .get_feedback_by_review(&review_id)
        .map_err(|e| e.to_string())?;
    Ok(feedbacks)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DiffSnippetLine {
    pub line_number: u32,
    pub content: String,
    pub prefix: String,
    pub is_addition: bool,
    pub is_deletion: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FeedbackDiffSnippet {
    pub file_path: String,
    pub hunk_header: String,
    pub lines: Vec<DiffSnippetLine>,
    pub highlighted_line: Option<u32>,
}

#[tauri::command]
pub fn get_feedback_diff_snippet(
    state: State<'_, AppState>,
    feedback_id: String,
    context_lines: u32,
) -> Result<Option<FeedbackDiffSnippet>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let feedback = db
        .feedback_repo()
        .find_by_id(&feedback_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Feedback not found".to_string())?;

    let anchor = match &feedback.anchor {
        Some(a) => a,
        None => return Ok(None),
    };

    let file_path = match &anchor.file_path {
        Some(f) => f.clone(),
        None => return Ok(None),
    };

    let line_number = match anchor.line_number {
        Some(l) => l,
        None => return Ok(None),
    };

    let side = match anchor.side {
        Some(crate::domain::FeedbackSide::Old) => crate::domain::FeedbackSide::Old,
        _ => crate::domain::FeedbackSide::New,
    };

    let review_id = &feedback.review_id;
    let review = db
        .get_review(review_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Review not found".to_string())?;

    let active_run_id = match &review.active_run_id {
        Some(id) => id.clone(),
        None => return Ok(None),
    };

    let review_run = db
        .get_review_run_by_id(&active_run_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Review run not found".to_string())?;

    let diff_index = crate::infra::diff::index::DiffIndex::new(&review_run.diff_text)
        .map_err(|e| e.to_string())?;

    let file_index = match diff_index.files.get(&file_path) {
        Some(f) => f,
        None => return Ok(None),
    };

    let mut target_hunk: Option<&crate::infra::diff::index::IndexedHunk> = None;
    for indexed_hunk in &file_index.all_hunks {
        let coords = indexed_hunk.coords;
        let hunk = &indexed_hunk.hunk;
        match side {
            crate::domain::FeedbackSide::Old => {
                let old_end = coords.0 + hunk.source_length.saturating_sub(1) as u32;
                if line_number >= coords.0 && line_number <= old_end {
                    target_hunk = Some(indexed_hunk);
                    break;
                }
            }
            crate::domain::FeedbackSide::New => {
                let new_end = coords.1 + hunk.target_length.saturating_sub(1) as u32;
                if line_number >= coords.1 && line_number <= new_end {
                    target_hunk = Some(indexed_hunk);
                    break;
                }
            }
        }
    }

    let indexed_hunk = match target_hunk {
        Some(h) => h,
        None => return Ok(None),
    };

    let coords = indexed_hunk.coords;
    let hunk = &indexed_hunk.hunk;
    let hunk_header = format!(
        "@@ -{},{} +{},{} @@",
        coords.0, hunk.source_length, coords.1, hunk.target_length
    );

    let mut snippet_lines: Vec<DiffSnippetLine> = Vec::new();

    let start_in_hunk = match side {
        crate::domain::FeedbackSide::Old => {
            if line_number >= coords.0 {
                Some((line_number - coords.0) as usize)
            } else {
                Some(0)
            }
        }
        crate::domain::FeedbackSide::New => {
            if line_number >= coords.1 {
                Some((line_number - coords.1) as usize)
            } else {
                Some(0)
            }
        }
    };

    let context_start = start_in_hunk
        .map(|s| s.saturating_sub(context_lines as usize))
        .unwrap_or(0);

    let context_end = start_in_hunk
        .map(|s| std::cmp::min(s + context_lines as usize + 1, hunk.target_length))
        .unwrap_or(hunk.target_length);

    let coords_clone = coords;
    let mut current_line_in_hunk = 0;
    crate::infra::diff::index::DiffIndex::walk_hunk_lines(
        hunk,
        coords_clone,
        |_pos, line, old_num, new_num| {
            if current_line_in_hunk >= context_start && current_line_in_hunk < context_end {
                let is_add = line.line_type.as_str() == unidiff::LINE_TYPE_ADDED;
                let is_del = line.line_type.as_str() == unidiff::LINE_TYPE_REMOVED;
                let prefix = if is_add {
                    "+"
                } else if is_del {
                    "-"
                } else {
                    " "
                };
                let display_line_number = match side {
                    crate::domain::FeedbackSide::Old => old_num,
                    crate::domain::FeedbackSide::New => new_num,
                };

                snippet_lines.push(DiffSnippetLine {
                    line_number: display_line_number.unwrap_or(0),
                    content: line.value.trim_end().to_string(),
                    prefix: prefix.to_string(),
                    is_addition: is_add,
                    is_deletion: is_del,
                });
            }
            current_line_in_hunk += 1;
        },
    );

    let highlighted_line = match side {
        crate::domain::FeedbackSide::Old => anchor.line_number,
        crate::domain::FeedbackSide::New => anchor.line_number,
    };

    Ok(Some(FeedbackDiffSnippet {
        file_path,
        hunk_header,
        lines: snippet_lines,
        highlighted_line,
    }))
}

#[tauri::command]
pub fn export_review(
    state: State<'_, AppState>,
    review_id: String,
    format: String,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let review = db
        .get_review(&review_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Review not found".to_string())?;

    let active_run_id = review.active_run_id.clone().unwrap_or_default();
    let tasks = db
        .get_tasks_by_run(&active_run_id)
        .map_err(|e| e.to_string())?;

    let feedbacks = db
        .get_feedback_by_review(&review_id)
        .map_err(|e| e.to_string())?;

    match format.as_str() {
        "markdown" => generate_markdown_export(&review, &tasks, &feedbacks),
        _ => Ok("exported_content".to_string()),
    }
}

fn generate_markdown_export(
    review: &Review,
    tasks: &[ReviewTask],
    feedbacks: &[Feedback],
) -> Result<String, String> {
    let mut md = format!("# {}\n\n", review.title);

    if let Some(summary) = &review.summary {
        md.push_str(summary);
        md.push_str("\n\n");
    }

    md.push_str("## Tasks\n\n");
    for task in tasks {
        let status_icon = match task.status {
            ReviewStatus::Todo => "[]",
            ReviewStatus::InProgress => "[ ]",
            ReviewStatus::Done => "[x]",
            ReviewStatus::Ignored => "[-]",
        };
        let risk = task.stats.risk.to_string();
        md.push_str(&format!("{} **{}** ({})\n", status_icon, task.title, risk));
        if !task.description.is_empty() {
            md.push_str(&format!(
                "> {}\n",
                task.description
                    .lines()
                    .take(3)
                    .collect::<Vec<_>>()
                    .join("\n> ")
            ));
        }
        md.push('\n');
    }

    if !feedbacks.is_empty() {
        md.push_str("## Feedback\n\n");
        for fb in feedbacks {
            let impact_icon = match fb.impact {
                FeedbackImpact::Blocking => "🔴",
                FeedbackImpact::NiceToHave => "🟡",
                FeedbackImpact::Nitpick => "🟢",
            };
            md.push_str(&format!("{} **{}**\n", impact_icon, fb.title));
            if let Some(anchor) = &fb.anchor
                && let Some(path) = &anchor.file_path
            {
                let line = anchor.line_number.unwrap_or(0);
                md.push_str(&format!("> At `{}`:{}\n", path, line));
            }
            md.push('\n');
        }
    }

    Ok(md)
}
#[tauri::command]
pub async fn fetch_remote_pr(
    _state: State<'_, AppState>,
    pr_ref: String,
    provider_hint: Option<String>,
) -> Result<ParsedDiff, String> {
    let registry = VcsRegistry::default();
    let provider = if let Some(hint) = provider_hint
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        let hint = hint.to_lowercase();
        registry
            .get_provider(&hint)
            .ok_or_else(|| format!("Unknown VCS provider: {}", hint))?
    } else {
        registry
            .detect_provider(&pr_ref)
            .ok_or_else(|| format!("Unsupported VCS reference: {}", pr_ref))?
    };

    let reference = provider
        .parse_ref(&pr_ref)
        .ok_or_else(|| format!("Invalid VCS reference: {}", pr_ref))?;

    let data = provider
        .fetch_pr(reference.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut parsed = parse_diff(data.diff_text)?;
    parsed.title = Some(data.title.clone());
    parsed.source = Some(data.source);

    Ok(parsed)
}

#[tauri::command]
pub async fn get_agents(_state: State<'_, AppState>) -> Result<Vec<AgentInfo>, String> {
    let candidates = crate::infra::acp::list_agent_candidates();

    let agents: Vec<AgentInfo> = candidates
        .into_iter()
        .map(|candidate| AgentInfo {
            id: candidate.id,
            name: candidate.label,
            description: None,
            path: candidate.command,
            args: candidate.args,
            logo: candidate.logo,
            available: candidate.available,
        })
        .collect();

    Ok(agents)
}

#[tauri::command]
pub fn update_agent_config(
    _state: State<'_, AppState>,
    id: String,
    path: String,
    args: Option<Vec<String>>,
) -> Result<(), String> {
    use crate::infra::app_config::{load_config, save_config};
    let mut config = load_config();

    // Check if it's a known built-in agent by checking its registry label/id
    // Actually, we can just check if it's in custom_agents first.
    let mut found_custom = false;
    for custom in config.custom_agents.iter_mut() {
        if custom.id == id {
            custom.command = path.clone();
            if let Some(new_args) = &args {
                custom.args = new_args.clone();
            }
            found_custom = true;
            break;
        }
    }

    if !found_custom {
        // Assume it's a built-in agent (or one we want to override)
        config.agent_path_overrides.insert(id.clone(), path);
        if let Some(new_args) = args {
            config.agent_args_overrides.insert(id, new_args);
        }
    }

    save_config(&config).map_err(|e| e.to_string())?;

    // Invalidate discovery cache so next get_agents or generation uses new path
    invalidate_agent_cache();

    Ok(())
}

#[tauri::command]
pub fn get_github_token() -> Result<Option<String>, String> {
    Ok(None)
}

#[tauri::command]
pub fn set_github_token(_token: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn get_vcs_status() -> Result<Vec<VcsStatus>, String> {
    let registry = VcsRegistry::default();
    let mut statuses = Vec::new();
    for provider in registry.providers() {
        let status = provider.get_status().await.map_err(|e| e.to_string())?;
        statuses.push(status);
    }
    Ok(statuses)
}

#[tauri::command]
pub async fn get_single_vcs_status(provider_id: String) -> Result<VcsStatus, String> {
    let registry = VcsRegistry::default();
    let provider = registry
        .get_provider(&provider_id)
        .ok_or_else(|| format!("Provider {} not found", provider_id))?;

    let status = provider.get_status().await.map_err(|e| e.to_string())?;
    Ok(status)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneRepoRequest {
    pub provider: String,
    pub repo: String,
    pub host: Option<String>,
    pub dest_dir: String,
}

fn link_repo_impl(state: &AppState, path: String) -> Result<LinkedRepo, String> {
    let id = Uuid::new_v4().to_string();
    let name = path.split('/').next_back().unwrap_or(&path).to_string();
    let linked_at = chrono::Utc::now().to_rfc3339();

    let domain_repo = DomainLinkedRepo {
        id: id.clone(),
        name: name.clone(),
        path: std::path::PathBuf::from(path.clone()),
        remotes: detect_remotes(&path),
        created_at: linked_at.clone(),
        allow_snapshot_access: false,
    };

    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.repo_repo()
        .save(&domain_repo)
        .map_err(|e| e.to_string())?;

    Ok(LinkedRepo {
        id,
        path,
        name,
        linked_at,
    })
}

#[tauri::command]
pub fn link_repo(state: State<'_, AppState>, path: String) -> Result<LinkedRepo, String> {
    link_repo_impl(state.inner(), path)
}

#[tauri::command]
pub async fn clone_and_link_repo(
    state: State<'_, AppState>,
    request: CloneRepoRequest,
) -> Result<LinkedRepo, String> {
    let provider = request.provider.trim().to_lowercase();
    let repo = request.repo.trim();
    let dest_dir = request.dest_dir.trim();

    if repo.is_empty() {
        return Err("Repository identifier is required".to_string());
    }

    if dest_dir.is_empty() {
        return Err("Destination directory is required".to_string());
    }

    let repo_name = repo.split('/').next_back().unwrap_or(repo);
    let dest_root = std::path::PathBuf::from(dest_dir);
    std::fs::create_dir_all(&dest_root)
        .map_err(|e| format!("Failed to create destination directory: {e}"))?;

    let target_path = dest_root.join(repo_name);
    if target_path.exists() {
        return Err(format!(
            "Destination already exists: {}",
            target_path.display()
        ));
    }

    let registry = VcsRegistry::default();
    let provider = registry
        .get_provider(&provider)
        .ok_or_else(|| format!("Unsupported provider: {}", request.provider))?;

    let clone_request = VcsCloneRequest {
        repo: repo.to_string(),
        dest_path: target_path.clone(),
        host: request.host.clone(),
    };

    provider
        .clone_repo(clone_request)
        .await
        .map_err(|e| e.to_string())?;

    link_repo_impl(state.inner(), target_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn unlink_repo(state: State<'_, AppState>, repo_id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.repo_repo().delete(&repo_id).map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewGenerationResult {
    pub task_count: usize,
    pub review_id: String,
    pub run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackInput {
    pub review_id: String,
    pub task_id: Option<String>,
    pub rule_id: Option<String>,
    pub title: String,
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
    pub side: Option<String>,
    pub content: String,
    pub impact: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    pub args: Vec<String>,
    pub logo: Option<String>,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInput {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: String,
    pub auto_refresh: bool,
    pub refresh_interval: u32,
    pub syntax_highlighting: bool,
    pub inline_comments: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingReviewState {
    pub id: String,
    pub diff: String,
    pub repo_root: Option<String>,
    pub agent: Option<String>,
    pub source: String,
    pub created_at: String,
    #[serde(default)]
    pub review_source: Option<ReviewSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewState {
    pub id: String,
    pub title: String,
    pub summary: Option<String>,
    pub agent_id: Option<String>,
    pub task_count: usize,
    pub created_at: String,
    pub source: crate::domain::ReviewSource,
    pub status: String,
    #[serde(default)]
    pub active_run_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRunState {
    pub id: String,
    pub review_id: String,
    pub agent_id: String,
    pub input_ref: String,
    pub diff_text: String,
    pub status: String,
    pub created_at: String,
    pub task_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedRepoState {
    pub id: String,
    pub name: String,
    pub path: String,
    pub review_count: usize,
    pub linked_at: String,
    pub remotes: Vec<String>,
    pub allow_snapshot_access: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorCandidate {
    pub id: String,
    pub label: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    pub preferred_editor_id: Option<String>,
}

#[tauri::command]
pub async fn get_available_editors() -> Vec<EditorCandidate> {
    crate::infra::editor::list_available_editors()
        .into_iter()
        .map(|e| EditorCandidate {
            id: e.id.to_string(),
            label: e.label.to_string(),
            path: e.path.to_string_lossy().to_string(),
        })
        .collect()
}

fn detect_remotes(path: &str) -> Vec<String> {
    use std::process::Command;
    let output = Command::new("git")
        .args(["remote", "-v"])
        .current_dir(path)
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut remotes = std::collections::HashSet::new();
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    remotes.insert(parts[1].to_string());
                }
            }
            remotes.into_iter().collect()
        }
        _ => Vec::new(),
    }
}

#[tauri::command]
pub fn get_editor_config() -> EditorConfig {
    use crate::infra::app_config::load_config;
    let config = load_config();
    EditorConfig {
        preferred_editor_id: config.preferred_editor_id,
    }
}

#[tauri::command]
pub fn update_editor_config(editor_id: String) -> Result<(), String> {
    use crate::infra::app_config::{load_config, save_config};
    let mut config = load_config();
    config.preferred_editor_id = Some(editor_id);
    save_config(&config).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackFilterConfig {
    pub confidence_threshold: Option<f64>,
}

#[tauri::command]
pub fn get_feedback_filter_config() -> FeedbackFilterConfig {
    use crate::infra::app_config::load_config;
    let config = load_config();
    FeedbackFilterConfig {
        confidence_threshold: config.feedback_confidence_threshold,
    }
}

#[tauri::command]
pub fn update_feedback_filter_config(threshold: Option<f64>) -> Result<(), String> {
    use crate::infra::app_config::{load_config, save_config};
    let mut config = load_config();
    // Clamp to valid range
    config.feedback_confidence_threshold = threshold.map(|t| t.clamp(0.0, 1.0));
    save_config(&config).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRuleInput {
    pub scope: String,
    pub repo_id: Option<String>,
    pub glob: Option<String>,
    pub category: Option<String>,
    pub text: String,
    pub enabled: bool,
}

#[tauri::command]
pub fn get_review_rules(state: State<'_, AppState>) -> Result<Vec<ReviewRule>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.rule_repo().list_all().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_review_rule(
    state: State<'_, AppState>,
    input: ReviewRuleInput,
) -> Result<ReviewRule, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let rule_id = Uuid::new_v4().to_string();
    let rule = build_review_rule(rule_id, now.clone(), now, input)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.rule_repo().save(&rule).map_err(|e| e.to_string())?;
    Ok(rule)
}

#[tauri::command]
pub fn update_review_rule(
    state: State<'_, AppState>,
    id: String,
    input: ReviewRuleInput,
) -> Result<ReviewRule, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let existing = db
        .rule_repo()
        .find_by_id(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Review rule not found".to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let rule = build_review_rule(id, existing.created_at, now, input)?;
    db.rule_repo().save(&rule).map_err(|e| e.to_string())?;
    Ok(rule)
}

#[tauri::command]
pub fn delete_review_rule(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.rule_repo().delete(&id).map_err(|e| e.to_string())?;
    Ok(())
}

fn build_review_rule(
    id: String,
    created_at: String,
    updated_at: String,
    input: ReviewRuleInput,
) -> Result<ReviewRule, String> {
    let scope = RuleScope::from_str(&input.scope.to_lowercase()).map_err(|e| e.to_string())?;
    let repo_id = input.repo_id.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    let glob = input.glob.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    let category = input.category.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    let text = input.text.trim().to_string();
    if text.is_empty() {
        return Err("Rule text cannot be empty".to_string());
    }

    match scope {
        RuleScope::Global => {
            if repo_id.is_some() {
                return Err("Global rules cannot target a repository".to_string());
            }
        }
        RuleScope::Repo => {
            if repo_id.is_none() {
                return Err("Repository rules require a repo_id".to_string());
            }
        }
    }

    Ok(ReviewRule {
        id,
        scope,
        repo_id,
        glob,
        category,
        text,
        enabled: input.enabled,
        created_at,
        updated_at,
    })
}

#[tauri::command]
pub fn open_in_editor(
    file_path: String,
    line_number: usize,
    repo_root: Option<String>,
) -> Result<(), String> {
    use crate::infra::app_config::load_config;
    use crate::infra::editor::{editor_command_for_open, is_editor_available};
    use std::path::PathBuf;
    use std::process::Command;

    let config = load_config();
    let editor_id = config
        .preferred_editor_id
        .ok_or_else(|| "No editor selected in settings".to_string())?;

    if !is_editor_available(&editor_id) {
        return Err(format!("Editor '{}' is not available", editor_id));
    }

    // Resolve absolute path if repo_root is provided
    let resolved_path = if let Some(root) = repo_root {
        PathBuf::from(root).join(&file_path)
    } else {
        PathBuf::from(&file_path)
    };

    if let Some((cmd_path, args)) = editor_command_for_open(&editor_id, &resolved_path, line_number)
    {
        Command::new(cmd_path)
            .args(args)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err(format!(
            "Could not construct open command for editor '{}'",
            editor_id
        ))
    }
}

/// Gets the local repository path for a given review by matching the review's source
/// (GitHub PR or GitLab MR) to a linked local repository via remote URLs.
#[tauri::command]
pub fn get_repo_root_for_review(
    state: State<'_, AppState>,
    review_id: String,
) -> Result<Option<String>, String> {
    use crate::domain::ReviewSource;

    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get the review to access its source
    let review = db
        .get_review(&review_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Review not found: {}", review_id))?;

    // Build the expected remote URL pattern based on source type
    let expected_patterns: Vec<String> = match &review.source {
        ReviewSource::GitHubPr { owner, repo, .. } => {
            vec![
                format!("github.com/{}/{}", owner, repo),
                format!("github.com:{}/{}", owner, repo),
            ]
        }
        ReviewSource::GitLabMr {
            host, project_path, ..
        } => {
            vec![
                format!("{}/{}", host, project_path),
                format!("{}:{}", host, project_path),
            ]
        }

        ReviewSource::DiffPaste { .. } => {
            // For pasted diffs, we can't auto-match to a repo
            return Ok(None);
        }
    };

    // Get all linked repos and search for a match
    let repos = db.get_linked_repos().map_err(|e| e.to_string())?;
    for repo in repos {
        for remote in &repo.remotes {
            // Normalize the remote URL for comparison
            let remote_lower = remote.to_lowercase();
            for pattern in &expected_patterns {
                if remote_lower.contains(&pattern.to_lowercase()) {
                    return Ok(Some(repo.path));
                }
            }
        }
    }

    Ok(None)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDiff {
    pub diff_text: String,
    pub total_additions: usize,
    pub total_deletions: usize,
    pub hunk_manifest: String,
    #[serde(default)]
    pub files: Vec<ParsedDiffFile>,
    #[serde(default)]
    pub source: Option<ReviewSource>,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDiffFile {
    pub name: String,
    pub old_path: String,
    pub new_path: String,
    pub hunks: Vec<ParsedHunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    #[serde(default)]
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedRepo {
    pub id: String,
    pub path: String,
    pub name: String,
    pub linked_at: String,
}

#[tauri::command]
pub async fn export_review_markdown(
    state: State<'_, AppState>,
    review_id: String,
    selected_tasks: Vec<String>,
    selected_feedbacks: Vec<String>,
) -> Result<String, String> {
    let data = {
        let db = state.db.lock().map_err(|e| e.to_string())?;

        let review = db
            .get_review(&review_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Review not found".to_string())?;

        let active_run_id = review
            .active_run_id
            .clone()
            .ok_or_else(|| "Review has no active run".to_string())?;

        let run = db
            .get_review_run_by_id(&active_run_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Review run not found".to_string())?;

        let tasks = db
            .get_tasks_by_run(&active_run_id)
            .map_err(|e| e.to_string())?;

        let feedbacks = db
            .get_feedback_by_review(&review_id)
            .map_err(|e| e.to_string())?
            // Filter out ignored feedbacks from export
            .into_iter()
            .filter(|f| f.status != ReviewStatus::Ignored)
            .collect::<Vec<_>>();

        let mut comments = Vec::new();
        for f in &feedbacks {
            let f_comments = db
                .get_comments_for_feedback(&f.id)
                .map_err(|e| e.to_string())?;
            comments.extend(f_comments);
        }

        ExportData {
            review,
            run,
            tasks,
            feedbacks,
            comments,
        }
    };

    let options = ExportOptions {
        include_summary: true,
        include_stats: true,
        include_metadata: true,
        include_tasks: true,
        include_feedbacks: true,
        include_context_diff: true,
        include_toc: true,
        selected_tasks: Some(selected_tasks.into_iter().collect()),
        selected_feedbacks: Some(selected_feedbacks.into_iter().collect()),
    };

    let result = ReviewExporter::export_to_markdown(&data, &options)
        .await
        .map_err(|e| e.to_string())?;

    Ok(result.markdown)
}

#[tauri::command]
pub async fn push_remote_review(
    state: State<'_, AppState>,
    review_id: String,
    selected_tasks: Vec<String>,
    selected_feedbacks: Vec<String>,
) -> Result<String, String> {
    let data = {
        let db = state.db.lock().map_err(|e| e.to_string())?;

        let review = db
            .get_review(&review_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Review not found".to_string())?;

        let active_run_id = review
            .active_run_id
            .clone()
            .ok_or_else(|| "Review has no active run".to_string())?;

        let run = db
            .get_review_run_by_id(&active_run_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Review run not found".to_string())?;

        let tasks = db
            .get_tasks_by_run(&active_run_id)
            .map_err(|e| e.to_string())?;

        let feedbacks = db
            .get_feedback_by_review(&review_id)
            .map_err(|e| e.to_string())?
            // Filter out ignored feedbacks from push to remote
            .into_iter()
            .filter(|f| f.status != ReviewStatus::Ignored)
            .collect::<Vec<_>>();

        let mut comments = Vec::new();
        for f in &feedbacks {
            let f_comments = db
                .get_comments_for_feedback(&f.id)
                .map_err(|e| e.to_string())?;
            comments.extend(f_comments);
        }

        ExportData {
            review,
            run,
            tasks,
            feedbacks,
            comments,
        }
    };

    let request = ReviewPushRequest {
        review: data.review,
        run: data.run,
        tasks: data.tasks,
        feedbacks: data.feedbacks,
        comments: data.comments,
        selected_tasks,
        selected_feedbacks,
    };

    let provider_id = request
        .review
        .source
        .provider_id()
        .ok_or_else(|| "Review has no remote provider".to_string())?;
    let registry = VcsRegistry::default();
    let provider = registry
        .get_provider(provider_id)
        .ok_or_else(|| format!("Unsupported VCS provider: {}", provider_id))?;

    provider
        .push_review(request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn push_remote_feedback(
    state: State<'_, AppState>,
    feedback_id: String,
) -> Result<String, String> {
    let (feedback, review, review_run, comments) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;

        let feedback = db
            .feedback_repo()
            .find_by_id(&feedback_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Feedback not found".to_string())?;

        let review = db
            .get_review(&feedback.review_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Review not found".to_string())?;

        let active_run_id = review
            .active_run_id
            .clone()
            .ok_or_else(|| "Review has no active run".to_string())?;

        let review_run = db
            .get_review_run_by_id(&active_run_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Review run not found".to_string())?;

        let comments = db
            .get_comments_for_feedback(&feedback_id)
            .map_err(|e| e.to_string())?;

        (feedback, review, review_run, comments)
    };

    let request = FeedbackPushRequest {
        review,
        run: review_run,
        feedback,
        comments,
    };

    let provider_id = request
        .review
        .source
        .provider_id()
        .ok_or_else(|| "Review has no remote provider".to_string())?;
    let registry = VcsRegistry::default();
    let provider = registry
        .get_provider(provider_id)
        .ok_or_else(|| format!("Unsupported VCS provider: {}", provider_id))?;

    provider
        .push_feedback(request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn copy_to_clipboard(text: String) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_pending_review_from_state(
    state: State<'_, AppState>,
) -> Result<Option<PendingReviewState>, String> {
    let pending_diff = state.pending_diff.lock().map_err(|e| e.to_string())?;
    Ok(pending_diff.as_ref().map(|p| PendingReviewState {
        id: uuid::Uuid::new_v4().to_string(),
        diff: p.diff.clone(),
        repo_root: p
            .repo_root
            .as_ref()
            .map(|r| r.to_string_lossy().to_string()),
        agent: p.agent.clone(),
        source: p.source.clone(),
        created_at: p.created_at.to_rfc3339(),
        review_source: None,
    }))
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", &url])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_diff_request(state: State<'_, AppState>) -> Result<Option<DiffRequestState>, String> {
    let diff_request = state.diff_request.lock().map_err(|e| e.to_string())?;
    Ok(diff_request.as_ref().map(|r| DiffRequestState {
        from: r.from.clone(),
        to: r.to.clone(),
        agent: r.agent.clone(),
        source: r.source.clone(),
    }))
}

#[tauri::command]
pub fn acquire_diff_from_request(state: State<'_, AppState>) -> Result<PendingReviewState, String> {
    let diff_request = {
        let request = state.diff_request.lock().map_err(|e| e.to_string())?;
        request
            .clone()
            .ok_or_else(|| "No diff request found".to_string())?
    };

    let mut review_source: Option<ReviewSource> = None;

    let diff = if let Ok(remote_ref) = crate::infra::cli::diff::parse_remote_ref(&diff_request.from)
    {
        match remote_ref {
            crate::infra::cli::diff::RemoteRef::GitHub {
                owner,
                repo,
                number,
            } => {
                let pr_url = format!("https://github.com/{}/{}/pull/{}", owner, repo, number);
                review_source = Some(ReviewSource::GitHubPr {
                    owner: owner.clone(),
                    repo: repo.clone(),
                    number,
                    url: Some(pr_url),
                    head_sha: None,
                    base_sha: None,
                });

                crate::infra::cli::diff::acquire_diff(
                    crate::infra::cli::diff::DiffSource::GitHubPr {
                        owner,
                        repo,
                        number,
                    },
                )
                .map_err(|e| e.to_string())?
            }
            crate::infra::cli::diff::RemoteRef::GitLab {
                host,
                project_path,
                number,
            } => {
                let url = format!("https://{host}/{project_path}/-/merge_requests/{number}");
                let mr_ref = crate::infra::vcs::gitlab::GitLabMrRef {
                    host: host.clone(),
                    project_path: project_path.clone(),
                    number,
                    url: url.clone(),
                };
                let metadata = tauri::async_runtime::block_on(
                    crate::infra::vcs::gitlab::fetch_mr_metadata(&mr_ref),
                )
                .ok();

                review_source = Some(ReviewSource::GitLabMr {
                    host: host.clone(),
                    project_path: project_path.clone(),
                    number,
                    url: Some(metadata.as_ref().map(|m| m.url.clone()).unwrap_or(url)),
                    head_sha: metadata.as_ref().and_then(|m| m.head_sha.clone()),
                    base_sha: metadata.as_ref().and_then(|m| m.base_sha.clone()),
                    start_sha: metadata.as_ref().and_then(|m| m.start_sha.clone()),
                });

                crate::infra::cli::diff::acquire_diff(
                    crate::infra::cli::diff::DiffSource::GitLabMr {
                        host,
                        project_path,
                        number,
                    },
                )
                .map_err(|e| e.to_string())?
            }
        }
    } else if diff_request.source == "uncommitted changes" {
        crate::infra::cli::diff::acquire_diff(crate::infra::cli::diff::DiffSource::GitStatus)
            .map_err(|e| e.to_string())?
    } else {
        let from = if diff_request.from.is_empty() {
            "HEAD".to_string()
        } else {
            diff_request.from.clone()
        };
        let to = if diff_request.to.is_empty() {
            "HEAD".to_string()
        } else {
            diff_request.to.clone()
        };
        crate::infra::cli::diff::acquire_diff(crate::infra::cli::diff::DiffSource::GitDiff {
            from,
            to,
        })
        .map_err(|e| e.to_string())?
    };

    let repo_root = crate::infra::cli::repo::detect_git_repo();

    let pending = PendingDiff {
        diff,
        repo_root,
        agent: diff_request.agent,
        source: diff_request.source.clone(),
        created_at: chrono::Utc::now(),
    };

    {
        let mut pending_diff = state.pending_diff.lock().map_err(|e| e.to_string())?;
        *pending_diff = Some(pending.clone());
    }

    {
        let mut diff_request = state.diff_request.lock().map_err(|e| e.to_string())?;
        *diff_request = None;
    }

    Ok(PendingReviewState {
        id: uuid::Uuid::new_v4().to_string(),
        diff: pending.diff,
        repo_root: pending.repo_root.map(|r| r.to_string_lossy().to_string()),
        agent: pending.agent,
        source: pending.source,
        created_at: pending.created_at.to_rfc3339(),
        review_source,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffRequestState {
    pub from: String,
    pub to: String,
    pub agent: Option<String>,
    pub source: String,
}

// ============================================================================
// Issue Check Commands
// ============================================================================

/// Get issue checks for a review run
#[tauri::command]
pub fn get_issue_checks_for_run(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<Vec<IssueCheckWithFindings>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let checks_with_findings = db
        .issue_check_repo()
        .find_checks_with_findings(&run_id)
        .map_err(|e| e.to_string())?;

    Ok(checks_with_findings
        .into_iter()
        .map(|(check, findings)| IssueCheckWithFindings { check, findings })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueCheckWithFindings {
    #[serde(flatten)]
    pub check: IssueCheck,
    pub findings: Vec<IssueFinding>,
}

use crate::domain::{IssueCheck, IssueFinding};

// ============================================================================
// Rule Library Commands
// ============================================================================

/// Get all rules from the library
#[tauri::command]
pub fn get_rule_library() -> Vec<LibraryRule> {
    LibraryRule::all()
}

/// Get library rules filtered by category
#[tauri::command]
pub fn get_rule_library_by_category(category: String) -> Vec<LibraryRule> {
    let category = match category.to_lowercase().as_str() {
        "security" => LibraryCategory::Security,
        "code_quality" | "codequality" => LibraryCategory::CodeQuality,
        "testing" => LibraryCategory::Testing,
        "documentation" => LibraryCategory::Documentation,
        "performance" => LibraryCategory::Performance,
        "api_design" | "apidesign" => LibraryCategory::ApiDesign,
        "language_specific" | "languagespecific" => LibraryCategory::LanguageSpecific,
        "framework_specific" | "frameworkspecific" => LibraryCategory::FrameworkSpecific,
        _ => return Vec::new(),
    };
    LibraryRule::by_category(category)
}

/// Add a rule from the library to the user's rules
#[tauri::command]
pub fn add_rule_from_library(
    state: State<'_, AppState>,
    library_rule_id: String,
    scope: String,
    repo_id: Option<String>,
) -> Result<ReviewRule, String> {
    // Find the library rule
    let library_rule = LibraryRule::all()
        .into_iter()
        .find(|r| r.id == library_rule_id)
        .ok_or_else(|| format!("Library rule not found: {}", library_rule_id))?;

    let now = chrono::Utc::now().to_rfc3339();
    let rule_id = Uuid::new_v4().to_string();

    let input = ReviewRuleInput {
        scope,
        repo_id,
        glob: library_rule.glob.clone(),
        category: library_rule.category.clone(),
        text: library_rule.text.clone(),
        enabled: true,
    };

    let rule = build_review_rule(rule_id, now.clone(), now, input)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.rule_repo().save(&rule).map_err(|e| e.to_string())?;
    Ok(rule)
}

use crate::domain::{LibraryCategory, LibraryRule};

/// Get default issue categories
#[tauri::command]
pub fn get_default_issue_categories() -> Vec<DefaultIssueCategory> {
    DefaultIssueCategory::defaults()
}

use crate::domain::DefaultIssueCategory;

// ============================================================================
// Rule Effectiveness / Analytics Commands
// ============================================================================

/// Statistics about rejection rates for a rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleRejectionStatsResponse {
    pub rule_id: String,
    pub total_feedback: i64,
    pub rejected_count: i64,
    pub rejection_rate: f64,
}

/// Get rejection statistics for all rules (for rule effectiveness dashboard)
#[tauri::command]
pub fn get_rule_rejection_stats(
    state: State<'_, AppState>,
) -> Result<Vec<RuleRejectionStatsResponse>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let stats = db
        .rejection_repo()
        .get_rule_stats()
        .map_err(|e| e.to_string())?;

    Ok(stats
        .into_iter()
        .map(|s| RuleRejectionStatsResponse {
            rule_id: s.rule_id,
            total_feedback: s.total_feedback,
            rejected_count: s.rejected_count,
            rejection_rate: s.rejection_rate,
        })
        .collect())
}

// ============================================================================
// Learning System Commands
// ============================================================================

use crate::domain::{
    LearnedPattern, LearnedPatternInput, LearningCompactionResult, LearningStatus,
};

/// Get all learned patterns
#[tauri::command]
pub fn get_learned_patterns(state: State<'_, AppState>) -> Result<Vec<LearnedPattern>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.learned_pattern_repo()
        .list_all()
        .map_err(|e| e.to_string())
}

/// Create a new learned pattern manually
#[tauri::command]
pub fn create_learned_pattern(
    state: State<'_, AppState>,
    input: LearnedPatternInput,
) -> Result<LearnedPattern, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.learned_pattern_repo()
        .create(&input, 0) // source_count = 0 for manual creation
        .map_err(|e| e.to_string())
}

/// Update an existing learned pattern
#[tauri::command]
pub fn update_learned_pattern(
    state: State<'_, AppState>,
    id: String,
    input: LearnedPatternInput,
) -> Result<LearnedPattern, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.learned_pattern_repo()
        .update(&id, &input)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Pattern not found".to_string())
}

/// Delete a learned pattern
#[tauri::command]
pub fn delete_learned_pattern(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let rows = db
        .learned_pattern_repo()
        .delete(&id)
        .map_err(|e| e.to_string())?;
    if rows == 0 {
        return Err("Pattern not found".to_string());
    }
    Ok(())
}

/// Toggle a learned pattern's enabled status
#[tauri::command]
pub fn toggle_learned_pattern(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let rows = db
        .learned_pattern_repo()
        .toggle_enabled(&id, enabled)
        .map_err(|e| e.to_string())?;
    if rows == 0 {
        return Err("Pattern not found".to_string());
    }
    Ok(())
}

/// Get the learning system status
#[tauri::command]
pub fn get_learning_status(state: State<'_, AppState>) -> Result<LearningStatus, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let pattern_repo = db.learned_pattern_repo();
    let state_repo = db.learning_state_repo();
    state_repo
        .get_status(&pattern_repo)
        .map_err(|e| e.to_string())
}

/// Trigger learning compaction manually
#[tauri::command]
pub async fn trigger_learning_compaction(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<LearningCompactionResult, String> {
    // Get the agent configuration
    let (agent_command, agent_args) = {
        let registry = AgentRegistry::default();
        let agent_candidate = registry
            .get_agent_candidate(&agent_id)
            .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;

        let command = agent_candidate.command.clone().ok_or_else(|| {
            format!(
                "Agent '{}' is not available. Please configure it in settings.",
                agent_id
            )
        })?;

        (command, agent_candidate.args.clone())
    };

    // Get unprocessed rejections and existing patterns
    let (rejections, existing_patterns, db_clone) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let rejection_repo = db.rejection_repo();
        let pattern_repo = db.learned_pattern_repo();

        let rejections = rejection_repo
            .get_unprocessed_rejections(50)
            .map_err(|e| e.to_string())?;

        let existing_patterns = pattern_repo.list_enabled().map_err(|e| e.to_string())?;

        // We need a clone of the Arc for the async operation
        drop(db);
        let db_clone = state.db.clone();

        (rejections, existing_patterns, db_clone)
    };

    if rejections.is_empty() {
        return Ok(LearningCompactionResult {
            rejections_processed: 0,
            patterns_created: 0,
            patterns_updated: 0,
            errors: vec!["No unprocessed rejections to analyze".to_string()],
        });
    }

    let input = crate::infra::acp::LearningCompactionInput {
        rejections,
        existing_patterns,
        agent_command,
        agent_args,
        db: db_clone,
        timeout_secs: Some(300),
        mcp_server_binary: None,
        cancel_token: None,
        debug: false,
    };

    crate::infra::acp::run_learning_compaction(input)
        .await
        .map_err(|e| e.to_string())
}
