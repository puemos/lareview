use crate::application::review::rules::resolve_rules;
use crate::domain::{ResolvedRule, Review, ReviewRun, ReviewRunStatus, ReviewSource, ReviewStatus};
use crate::infra::acp::{
    AgentConfigSelection, GenerateTasksInput, ProgressEvent, RunContext, generate_tasks_with_acp,
    list_agent_candidates,
};
use crate::infra::db::Database;
use crate::infra::diff::index::DiffIndex;
use crate::infra::hash::hash_diff;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{Semaphore, mpsc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub const REVIEW_RUN_EVENT: &str = "review-run-event";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum ProgressEventPayload {
    Status {
        status: String,
    },
    Log(String),
    MessageDelta {
        id: String,
        delta: String,
    },
    ThoughtDelta {
        id: String,
        delta: String,
    },
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
    manager: Option<crate::infra::vcs::snapshot::SnapshotManager>,
    path: Option<std::path::PathBuf>,
}

impl SnapshotCleanupGuard {
    fn new(
        manager: Option<crate::infra::vcs::snapshot::SnapshotManager>,
        path: Option<std::path::PathBuf>,
    ) -> Self {
        Self { manager, path }
    }
}

impl Drop for SnapshotCleanupGuard {
    fn drop(&mut self) {
        if let (Some(manager), Some(path)) = (&self.manager, &self.path)
            && path.exists()
            && let Err(error) = manager.remove_blocking(path)
        {
            log::warn!(
                "Failed to clean up snapshot {} from scope guard: {error}",
                path.display()
            );
        }
    }
}

async fn cleanup_snapshot(
    manager: &crate::infra::vcs::snapshot::SnapshotManager,
    path: &std::path::Path,
) {
    let mut retries = 5;
    let mut delay = std::time::Duration::from_millis(200);

    loop {
        if !path.exists() {
            break;
        }

        if let Err(error) = manager.remove(path).await {
            log::warn!("Failed to cleanup snapshot {}: {}", path.display(), error);
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

#[derive(Clone)]
struct GenerationJob {
    review_id: String,
    run_id: String,
    diff_text: String,
    agent_id: String,
    candidate_label: String,
    command: String,
    candidate_args: Vec<String>,
    repo_id: Option<String>,
    source: ReviewSource,
    use_snapshot: bool,
    agent_config: Vec<AgentConfigSelection>,
    created_at: String,
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn generate_review(
    app: AppHandle,
    state: State<'_, AppState>,
    diff_text: String,
    agent_id: String,
    repo_id: Option<String>,
    source: Option<ReviewSource>,
    use_snapshot: bool,
    agent_config: Option<Vec<AgentConfigSelection>>,
) -> Result<ReviewGenerationResult, String> {
    let diff_hash = hash_diff(&diff_text);
    DiffIndex::new(&diff_text).map_err(|error| format!("Invalid diff: {error}"))?;

    let (candidate_label, command, candidate_args) = {
        let candidates = list_agent_candidates();
        let candidate = candidates
            .iter()
            .find(|candidate| candidate.id == agent_id)
            .or_else(|| {
                candidates
                    .iter()
                    .find(|candidate| candidate.id == "default")
            })
            .ok_or_else(|| "No agent found".to_string())?;
        let command = candidate.command.clone().ok_or_else(|| {
            format!(
                "Agent '{}' is not available. Please configure the agent path in settings.",
                agent_id
            )
        })?;
        (candidate.label.clone(), command, candidate.args.clone())
    };

    let review_id = Uuid::new_v4().to_string();
    let run_id = Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();
    let source = source.unwrap_or_else(|| ReviewSource::DiffPaste {
        diff_hash: diff_hash.clone(),
    });
    let initial_title = match &source {
        ReviewSource::GitHubPr { repo, number, .. } => format!("PR {repo}#{number}"),
        ReviewSource::GitLabMr {
            project_path,
            number,
            ..
        } => format!("MR {project_path}!{number}"),
        ReviewSource::DiffPaste { .. } => "AI Review".to_string(),
    };

    let review = Review {
        id: review_id.clone(),
        title: initial_title,
        summary: None,
        source: source.clone(),
        active_run_id: Some(run_id.clone()),
        status: ReviewStatus::Todo,
        created_at: created_at.clone(),
        updated_at: created_at.clone(),
    };
    let run = ReviewRun {
        id: run_id.clone(),
        review_id: review_id.clone(),
        agent_id: agent_id.clone(),
        input_ref: format!("diff-{}", &diff_hash[..8]),
        diff_text: Arc::from(diff_text.as_str()),
        diff_hash,
        status: ReviewRunStatus::Queued,
        created_at: created_at.clone(),
    };

    {
        let db = state.db.lock().map_err(|error| error.to_string())?;
        db.save_review_with_run(&review, &run)
            .map_err(|error| error.to_string())?;
    }

    let cancel_token = CancellationToken::new();
    state
        .active_runs
        .lock()
        .map_err(|error| error.to_string())?
        .insert(run_id.clone(), cancel_token.clone());

    emit_progress(
        &state.db,
        &app,
        &review_id,
        &run_id,
        ProgressEventPayload::Status {
            status: ReviewRunStatus::Queued.to_string(),
        },
    );

    let job = GenerationJob {
        review_id: review_id.clone(),
        run_id: run_id.clone(),
        diff_text,
        agent_id,
        candidate_label,
        command,
        candidate_args,
        repo_id: repo_id.and_then(|id| {
            let trimmed = id.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }),
        source,
        use_snapshot,
        agent_config: agent_config.unwrap_or_default(),
        created_at,
    };

    let db = state.db.clone();
    let active_runs = state.active_runs.clone();
    let generation_slots = state.generation_slots.clone();
    tauri::async_runtime::spawn(async move {
        run_generation_job(app, db, active_runs, generation_slots, cancel_token, job).await;
    });

    Ok(ReviewGenerationResult {
        review_id,
        run_id,
        status: ReviewRunStatus::Queued,
    })
}

async fn run_generation_job(
    app: AppHandle,
    db: Arc<Mutex<Database>>,
    active_runs: Arc<Mutex<std::collections::HashMap<String, CancellationToken>>>,
    generation_slots: Arc<Semaphore>,
    cancel_token: CancellationToken,
    job: GenerationJob,
) {
    let permit = tokio::select! {
        _ = cancel_token.cancelled() => {
            finish_run(
                &db,
                &app,
                &job,
                ReviewRunStatus::Queued,
                ReviewRunStatus::Cancelled,
                Some("Generation cancelled by user"),
            );
            remove_active_run(&active_runs, &job.run_id);
            return;
        }
        permit = generation_slots.acquire_owned() => match permit {
            Ok(permit) => permit,
            Err(error) => {
                finish_run(
                    &db,
                    &app,
                    &job,
                    ReviewRunStatus::Queued,
                    ReviewRunStatus::Failed,
                    Some(&format!("Generation queue closed: {error}")),
                );
                remove_active_run(&active_runs, &job.run_id);
                return;
            }
        }
    };

    if cancel_token.is_cancelled() {
        finish_run(
            &db,
            &app,
            &job,
            ReviewRunStatus::Queued,
            ReviewRunStatus::Cancelled,
            Some("Generation cancelled by user"),
        );
        remove_active_run(&active_runs, &job.run_id);
        drop(permit);
        return;
    }

    let started = match db.lock() {
        Ok(database) => database.transition_run_status(
            &job.run_id,
            ReviewRunStatus::Queued,
            ReviewRunStatus::Running,
        ),
        Err(error) => Err(rusqlite::Error::InvalidParameterName(error.to_string())),
    };
    match started {
        Ok(true) => {}
        Ok(false) => {
            log::warn!(
                "Run {} left queued state before its generation slot was acquired",
                job.run_id
            );
            remove_active_run(&active_runs, &job.run_id);
            drop(permit);
            return;
        }
        Err(error) => {
            log::error!("Failed to mark run {} as running: {error}", job.run_id);
            finish_run(
                &db,
                &app,
                &job,
                ReviewRunStatus::Queued,
                ReviewRunStatus::Failed,
                Some("Failed to start the queued review"),
            );
            remove_active_run(&active_runs, &job.run_id);
            drop(permit);
            return;
        }
    }
    emit_progress(
        &db,
        &app,
        &job.review_id,
        &job.run_id,
        ProgressEventPayload::Status {
            status: ReviewRunStatus::Running.to_string(),
        },
    );

    let result = execute_generation(&app, &db, &cancel_token, &job).await;
    match result {
        Ok(task_count) => {
            if finish_run(
                &db,
                &app,
                &job,
                ReviewRunStatus::Running,
                ReviewRunStatus::Completed,
                None,
            ) {
                emit_progress(
                    &db,
                    &app,
                    &job.review_id,
                    &job.run_id,
                    ProgressEventPayload::Completed { task_count },
                );
            }
        }
        Err(error) => {
            let cancelled =
                cancel_token.is_cancelled() || error.to_lowercase().contains("cancelled by user");
            let status = if cancelled {
                ReviewRunStatus::Cancelled
            } else {
                ReviewRunStatus::Failed
            };
            finish_run(
                &db,
                &app,
                &job,
                ReviewRunStatus::Running,
                status,
                Some(&error),
            );
        }
    }

    remove_active_run(&active_runs, &job.run_id);
    drop(permit);
}

async fn execute_generation(
    app: &AppHandle,
    db: &Arc<Mutex<Database>>,
    cancel_token: &CancellationToken,
    job: &GenerationJob,
) -> Result<usize, String> {
    let mut snapshot_manager = None;
    let snapshot_path = if job.use_snapshot {
        let head_sha = job.source.head_sha();
        if let (Some(repo_id), Some(head_sha)) = (job.repo_id.as_ref(), head_sha.as_deref()) {
            let repos = {
                let database = db.lock().map_err(|error| error.to_string())?;
                database
                    .get_linked_repos()
                    .map_err(|error| error.to_string())?
            };
            if let Some(repo) = repos.iter().find(|repo| repo.id == *repo_id) {
                let manager = crate::infra::vcs::snapshot::SnapshotManager::new(
                    std::path::PathBuf::from(&repo.path),
                );
                let short_sha: String = head_sha.chars().take(7).collect();
                emit_progress(
                    db,
                    app,
                    &job.review_id,
                    &job.run_id,
                    ProgressEventPayload::Log(format!(
                        "Creating snapshot for {} at {short_sha}...",
                        repo.name
                    )),
                );
                let path = manager
                    .create(&job.run_id, head_sha)
                    .await
                    .map_err(|error| error.to_string())?;
                emit_progress(
                    db,
                    app,
                    &job.review_id,
                    &job.run_id,
                    ProgressEventPayload::Log("Snapshot ready".to_string()),
                );
                snapshot_manager = Some(manager);
                Some(path)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    let _snapshot_guard =
        SnapshotCleanupGuard::new(snapshot_manager.clone(), snapshot_path.clone());

    emit_progress(
        db,
        app,
        &job.review_id,
        &job.run_id,
        ProgressEventPayload::Log(format!(
            "Starting review generation with {}...",
            job.candidate_label
        )),
    );

    let diff_hash = hash_diff(&job.diff_text);
    let run_context = RunContext {
        review_id: job.review_id.clone(),
        run_id: job.run_id.clone(),
        agent_id: job.agent_id.clone(),
        input_ref: format!("diff-{}", &diff_hash[..8]),
        diff_text: Arc::from(job.diff_text.as_str()),
        diff_hash,
        source: job.source.clone(),
        initial_title: None,
        created_at: Some(job.created_at.clone()),
    };

    let diff_paths = DiffIndex::new(&job.diff_text)
        .map(|index| index.get_all_file_paths())
        .unwrap_or_default();
    let rules: Vec<ResolvedRule> = {
        let database = db.lock().map_err(|error| error.to_string())?;
        let all_rules = database
            .rule_repo()
            .list_enabled()
            .map_err(|error| error.to_string())?;
        resolve_rules(&all_rules, job.repo_id.as_deref(), &diff_paths)
    };

    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<ProgressEvent>();
    let progress_db = db.clone();
    let progress_app = app.clone();
    let progress_review_id = job.review_id.clone();
    let progress_run_id = job.run_id.clone();
    let progress_forwarder = tauri::async_runtime::spawn(async move {
        while let Some(event) = progress_rx.recv().await {
            let payload = progress_payload(event);
            emit_progress(
                &progress_db,
                &progress_app,
                &progress_review_id,
                &progress_run_id,
                payload,
            );
        }
    });

    let result = generate_tasks_with_acp(GenerateTasksInput {
        run_context,
        rules,
        repo_root: snapshot_path.clone(),
        cleanup_path: snapshot_path.clone(),
        agent_command: job.command.clone(),
        agent_args: job.candidate_args.clone(),
        agent_config: job.agent_config.clone(),
        progress_tx: Some(progress_tx),
        mcp_server_binary: None,
        timeout_secs: Some(
            crate::infra::app_config::load_config()
                .review_timeout_secs
                .unwrap_or(1000),
        ),
        cancel_token: Some(cancel_token.clone()),
        debug: std::env::var("RUST_LOG")
            .map(|value| value.contains("acp"))
            .unwrap_or(false),
    })
    .await;

    if let Err(error) = progress_forwarder.await {
        log::warn!("Progress forwarder for {} failed: {error}", job.run_id);
    }

    if let (Some(manager), Some(path)) = (snapshot_manager.as_ref(), snapshot_path.as_ref()) {
        cleanup_snapshot(manager, path).await;
    }

    result.map_err(|error| error.to_string())?;
    let task_count = {
        let database = db.lock().map_err(|error| error.to_string())?;
        database
            .get_tasks_by_run(&job.run_id)
            .map(|tasks| tasks.len())
            .unwrap_or(0)
    };
    Ok(task_count)
}

fn progress_payload(event: ProgressEvent) -> ProgressEventPayload {
    match event {
        ProgressEvent::LocalLog(message) => ProgressEventPayload::Log(message),
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
            let plan_value = serde_json::to_value(plan).unwrap_or_default();
            let entries = plan_value
                .get("entries")
                .and_then(serde_json::Value::as_array)
                .map(|entries| {
                    entries
                        .iter()
                        .map(|entry| FrontendPlanEntry {
                            content: entry
                                .get("content")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                            priority: entry
                                .get("priority")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("medium")
                                .to_string(),
                            status: entry
                                .get("status")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("pending")
                                .to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            ProgressEventPayload::Plan(FrontendPlan { entries })
        }
        ProgressEvent::TaskStarted(task_id, title) => {
            ProgressEventPayload::TaskStarted { task_id, title }
        }
        ProgressEvent::TaskAdded(task_id) => ProgressEventPayload::TaskCompleted { task_id },
        ProgressEvent::FeedbackAdded => ProgressEventPayload::Log("Feedback added".to_string()),
        ProgressEvent::MetadataUpdated => {
            ProgressEventPayload::Log("Review details updated".to_string())
        }
        ProgressEvent::Finalized => {
            ProgressEventPayload::Log("Agent finalized the review".to_string())
        }
    }
}

fn finish_run(
    db: &Arc<Mutex<Database>>,
    app: &AppHandle,
    job: &GenerationJob,
    expected_status: ReviewRunStatus,
    status: ReviewRunStatus,
    message: Option<&str>,
) -> bool {
    let transitioned = match db.lock() {
        Ok(database) => database.transition_run_status(&job.run_id, expected_status, status),
        Err(error) => {
            log::error!(
                "Failed to lock database while finishing {}: {error}",
                job.run_id
            );
            return false;
        }
    };
    match transitioned {
        Ok(true) => {}
        Ok(false) => {
            log::warn!(
                "Ignored stale transition for run {} from {} to {}",
                job.run_id,
                expected_status,
                status
            );
            return false;
        }
        Err(error) => {
            log::error!("Failed to finish run {}: {error}", job.run_id);
            return false;
        }
    }

    let payload = match status {
        ReviewRunStatus::Failed | ReviewRunStatus::Cancelled | ReviewRunStatus::Interrupted => {
            ProgressEventPayload::Error {
                message: message.unwrap_or("Generation stopped").to_string(),
            }
        }
        _ => ProgressEventPayload::Status {
            status: status.to_string(),
        },
    };
    emit_progress(db, app, &job.review_id, &job.run_id, payload);
    true
}

fn emit_progress(
    db: &Arc<Mutex<Database>>,
    app: &AppHandle,
    review_id: &str,
    run_id: &str,
    payload: ProgressEventPayload,
) {
    let payload_value = match serde_json::to_value(&payload) {
        Ok(value) => value,
        Err(error) => {
            log::warn!("Failed to serialize activity for {run_id}: {error}");
            return;
        }
    };
    let event = match db.lock() {
        Ok(database) => database
            .save_review_run_event(review_id, run_id, &payload_value)
            .map_err(|error| error.to_string()),
        Err(error) => Err(error.to_string()),
    };

    match event {
        Ok(event) => {
            if let Err(error) = app.emit(REVIEW_RUN_EVENT, event) {
                log::warn!("Failed to emit activity for {run_id}: {error}");
            }
        }
        Err(error) => log::warn!("Failed to persist activity for {run_id}: {error}"),
    }
}

fn remove_active_run(
    active_runs: &Arc<Mutex<std::collections::HashMap<String, CancellationToken>>>,
    run_id: &str,
) {
    if let Ok(mut active) = active_runs.lock() {
        active.remove(run_id);
    }
}

#[tauri::command]
pub async fn stop_generation(state: State<'_, AppState>, run_id: String) -> Result<(), String> {
    let token = state
        .active_runs
        .lock()
        .map_err(|error| error.to_string())?
        .get(&run_id)
        .cloned();
    if let Some(token) = token {
        token.cancel();
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewGenerationResult {
    pub review_id: String,
    pub run_id: String,
    pub status: ReviewRunStatus,
}
