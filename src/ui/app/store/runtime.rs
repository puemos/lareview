use super::action::{Action, AsyncAction, ReviewAction, ReviewDataPayload, ReviewThreadsPayload};
use super::command::{Command, D2Command};
use crate::domain::{Comment, ReviewId, Thread, ThreadAnchor, ThreadImpact, ThreadStatus};
use crate::infra::acp::{GenerateTasksInput, generate_tasks_with_acp, list_agent_candidates};
use crate::ui::app::{GenMsg, GenResultPayload};
use crate::ui::app::{GhMsg, GhStatusPayload};

use super::super::LaReviewApp;
use std::collections::HashMap;

pub fn run(app: &mut LaReviewApp, command: Command) {
    match command {
        Command::ResolveGenerateInput {
            input_text,
            selected_agent_id,
            review_id,
        } => resolve_generate_input(app, input_text, selected_agent_id, review_id),
        Command::FetchPrContextPreview { input_ref } => fetch_pr_context_preview(app, input_ref),
        Command::CheckGitHubStatus => check_github_status(app),
        Command::RefreshGitHubReview {
            review_id,
            selected_agent_id,
        } => refresh_github_review(app, review_id, selected_agent_id),
        Command::StartGeneration {
            run_context,
            selected_agent_id,
        } => start_generation(app, *run_context, selected_agent_id),
        Command::RefreshReviewData { reason } => refresh_review_data(app, reason),
        Command::LoadReviewThreads { review_id } => load_review_threads(app, review_id),
        Command::UpdateTaskStatus { task_id, status } => update_task_status(app, task_id, status),
        Command::DeleteReview { review_id } => delete_review(app, review_id),
        Command::CreateThreadComment {
            review_id,
            task_id,
            thread_id,
            file_path,
            line_number,
            title,
            body,
        } => create_thread_comment(
            app,
            review_id,
            task_id,
            thread_id,
            file_path,
            line_number,
            title,
            body,
        ),
        Command::RunD2 { command } => run_d2_command(app, command),
        Command::GenerateExportPreview { review_id, run_id } => {
            generate_export_preview(app, review_id, run_id)
        }
        Command::ExportReview {
            review_id,
            run_id,
            path,
        } => export_review(app, review_id, run_id, path),
        Command::UpdateThreadStatus { thread_id, status } => {
            update_thread_status(app, thread_id, status)
        }
        Command::UpdateThreadImpact { thread_id, impact } => {
            update_thread_impact(app, thread_id, impact)
        }
        Command::UpdateThreadTitle { thread_id, title } => {
            update_thread_title(app, thread_id, title)
        }
        Command::SaveRepo { repo } => save_repo(app, repo),
        Command::DeleteRepo { repo_id } => delete_repo(app, repo_id),
        Command::PickFolderForLink => pick_folder_for_link(app),
        Command::SaveAppConfig {
            extra_path,
            has_seen_requirements,
        } => save_app_config(extra_path, has_seen_requirements),
    }
}

fn resolve_generate_input(
    app: &mut LaReviewApp,
    input_text: String,
    selected_agent_id: String,
    review_id: Option<String>,
) {
    let gen_tx = app.gen_tx.clone();

    tokio::spawn(async move {
        let result = crate::ui::app::generate_input::resolve_generate_input(
            input_text,
            selected_agent_id,
            review_id,
        )
        .await
        .map_err(|e| e.to_string());

        let _ = gen_tx
            .send(crate::ui::app::GenMsg::InputResolved(Box::new(result)))
            .await;
    });
}

fn fetch_pr_context_preview(app: &mut LaReviewApp, input_ref: String) {
    let gen_tx = app.gen_tx.clone();

    tokio::spawn(async move {
        let result = crate::ui::app::generate_input::resolve_pr_preview(input_ref.clone())
            .await
            .map_err(|e| e.to_string());

        let _ = gen_tx
            .send(crate::ui::app::GenMsg::PreviewResolved { input_ref, result })
            .await;
    });
}

fn refresh_github_review(app: &mut LaReviewApp, review_id: String, selected_agent_id: String) {
    let Some(review) = app
        .state
        .reviews
        .iter()
        .find(|r| r.id == review_id)
        .cloned()
    else {
        let _ = app
            .gen_tx
            .try_send(crate::ui::app::GenMsg::InputResolved(Box::new(Err(
                "Selected review not found".into(),
            ))));
        return;
    };

    let gen_tx = app.gen_tx.clone();

    tokio::spawn(async move {
        let result =
            crate::ui::app::generate_input::resolve_github_refresh(&review, selected_agent_id)
                .await
                .map_err(|e| e.to_string());

        let _ = gen_tx
            .send(crate::ui::app::GenMsg::InputResolved(Box::new(result)))
            .await;
    });
}

fn check_github_status(app: &mut LaReviewApp) {
    let gh_tx = app.gh_tx.clone();

    tokio::spawn(async move {
        let result: Result<GhStatusPayload, String> = async {
            let gh_path = crate::infra::brew::find_bin("gh")
                .ok_or_else(|| "gh is not installed".to_string())?;

            let auth = tokio::process::Command::new(&gh_path)
                .args(["auth", "status", "--hostname", "github.com"])
                .output()
                .await
                .map_err(|e| format!("Failed to run `gh auth status`: {e}"))?;

            if !auth.status.success() {
                let stderr = String::from_utf8_lossy(&auth.stderr).trim().to_string();
                return Err(if stderr.is_empty() {
                    "Not authenticated. Run: gh auth login".to_string()
                } else {
                    format!("Not authenticated. gh: {stderr}")
                });
            }

            let whoami = tokio::process::Command::new(&gh_path)
                .args(["api", "user", "-q", ".login"])
                .output()
                .await
                .map_err(|e| format!("Failed to run `gh api user`: {e}"))?;

            let login = if whoami.status.success() {
                String::from_utf8(whoami.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            } else {
                None
            };

            Ok(GhStatusPayload {
                gh_path: gh_path.display().to_string(),
                login,
            })
        }
        .await;

        let _ = gh_tx.send(GhMsg::Done(result)).await;
    });
}

fn start_generation(
    app: &mut LaReviewApp,
    run_context: crate::infra::acp::RunContext,
    selected_agent_id: String,
) {
    let candidates: Vec<_> = list_agent_candidates()
        .into_iter()
        .filter(|c| c.available)
        .collect();

    let Some(candidate) = candidates
        .iter()
        .find(|c| c.id == selected_agent_id)
        .cloned()
        .or_else(|| candidates.first().cloned())
    else {
        let _ = app.gen_tx.try_send(GenMsg::Done(Err(
            "No ACP agents available on this system".into()
        )));
        return;
    };

    let Some(agent_command) = candidate.command.clone() else {
        let _ = app.gen_tx.try_send(GenMsg::Done(Err(format!(
            "Selected agent '{}' is not executable on this system",
            candidate.id
        ))));
        return;
    };

    let _ = app.gen_tx.try_send(GenMsg::Progress(Box::new(
        crate::infra::acp::ProgressEvent::LocalLog(format!("starting agent: {}", candidate.label)),
    )));

    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();

    let start_log = format!(
        "agent: {} ({} {})",
        candidate.id,
        agent_command,
        candidate.args.join(" ")
    );

    let repo_root = app
        .state
        .selected_repo_id
        .as_ref()
        .and_then(|id| app.state.linked_repos.iter().find(|r| &r.id == id))
        .map(|r| r.path.clone());

    let input = GenerateTasksInput {
        run_context,
        repo_root,
        agent_command,
        agent_args: candidate.args,
        progress_tx: Some(progress_tx),
        mcp_server_binary: None,
        timeout_secs: Some(5000),
        debug: std::env::var("ACP_DEBUG").is_ok(),
    };

    let gen_tx = app.gen_tx.clone();

    tokio::spawn(async move {
        let mut result_fut = std::pin::pin!(generate_tasks_with_acp(input));

        loop {
            tokio::select! {
                evt = progress_rx.recv() => {
                    if let Some(evt) = evt {
                        let _ = gen_tx.send(GenMsg::Progress(Box::new(evt))).await;
                    }
                }
                res = &mut result_fut => {
                    let msg = match res {
                        Ok(res) => {
                            let mut logs = res.logs;
                            logs.insert(0, start_log.clone());
                            GenMsg::Done(Ok(GenResultPayload {
                                messages: res.messages,
                                thoughts: res.thoughts,
                                logs,
                            }))
                        }
                        Err(e) => GenMsg::Done(Err(format!("Generation failed: {e}"))),
                    };

                    let _ = gen_tx.send(msg).await;
                    break;
                }
            }
        }
    });
}

fn save_app_config(extra_path: String, has_seen_requirements: bool) {
    let extra_path = extra_path.trim().to_string();
    let config = crate::infra::app_config::AppConfig {
        extra_path: if extra_path.is_empty() {
            None
        } else {
            Some(extra_path.clone())
        },
        has_seen_requirements,
    };

    if !extra_path.is_empty() {
        // set_var is currently unsafe on nightly; this is limited to process-local config.
        unsafe {
            std::env::set_var("LAREVIEW_EXTRA_PATH", extra_path);
        }
    } else {
        unsafe {
            std::env::remove_var("LAREVIEW_EXTRA_PATH");
        }
    }

    if let Err(err) = crate::infra::app_config::save_config(&config) {
        eprintln!("[config] Failed to save config: {err}");
    }
}

fn refresh_review_data(app: &mut LaReviewApp, reason: super::command::ReviewDataRefreshReason) {
    let result = (|| -> Result<ReviewDataPayload, String> {
        let reviews = app
            .review_repo
            .list_all()
            .map_err(|e| format!("Failed to load reviews: {e}"))?;
        let runs = app
            .run_repo
            .list_all()
            .map_err(|e| format!("Failed to load review runs: {e}"))?;
        let tasks = app
            .task_repo
            .find_all()
            .map_err(|e| format!("Failed to load tasks: {e}"))?;
        Ok(ReviewDataPayload {
            reviews,
            runs,
            tasks,
        })
    })();

    app.dispatch(Action::Async(AsyncAction::ReviewDataLoaded {
        reason,
        result,
    }));
}

fn load_review_threads(app: &mut LaReviewApp, review_id: ReviewId) {
    let result = (|| -> Result<ReviewThreadsPayload, String> {
        let threads = app
            .thread_repo
            .find_by_review(&review_id)
            .map_err(|e| format!("Failed to load threads: {e}"))?;
        let mut comments = HashMap::new();
        for thread in &threads {
            let thread_comments = app
                .comment_repo
                .list_for_thread(&thread.id)
                .map_err(|e| format!("Failed to load comments: {e}"))?;
            comments.insert(thread.id.clone(), thread_comments);
        }
        Ok(ReviewThreadsPayload {
            review_id,
            threads,
            comments,
        })
    })();

    app.dispatch(Action::Async(AsyncAction::ReviewThreadsLoaded(result)));
}

fn update_task_status(
    app: &mut LaReviewApp,
    task_id: crate::domain::TaskId,
    status: crate::domain::TaskStatus,
) {
    let result = app
        .task_repo
        .update_status(&task_id, status)
        .map_err(|e| format!("Failed to update task status: {e}"));

    app.dispatch(Action::Async(AsyncAction::TaskStatusSaved(result)));
}

fn delete_review(app: &mut LaReviewApp, review_id: ReviewId) {
    let result = (|| -> Result<(), String> {
        let runs = app
            .run_repo
            .find_by_review_id(&review_id)
            .map_err(|e| format!("Failed to fetch runs for review: {e}"))?;

        if !runs.is_empty() {
            let run_ids: Vec<_> = runs.iter().map(|r| r.id.clone()).collect();
            let tasks = app
                .task_repo
                .find_by_run_ids(&run_ids)
                .map_err(|e| format!("Failed to fetch tasks for runs: {e}"))?;

            if !tasks.is_empty() {
                let task_ids: Vec<_> = tasks.iter().map(|t| t.id.clone()).collect();

                app.task_repo
                    .delete_by_ids(&task_ids)
                    .map_err(|e| format!("Failed to delete tasks: {e}"))?;
            }

            app.run_repo
                .delete_by_review_id(&review_id)
                .map_err(|e| format!("Failed to delete runs: {e}"))?;
        }

        app.review_repo
            .delete(&review_id)
            .map_err(|e| format!("Failed to delete review: {e}"))?;

        Ok(())
    })();
    app.dispatch(Action::Async(AsyncAction::ReviewDeleted(result)));
}

#[allow(clippy::too_many_arguments)]
fn create_thread_comment(
    app: &mut LaReviewApp,
    review_id: ReviewId,
    task_id: crate::domain::TaskId,
    thread_id: Option<String>,
    file_path: Option<String>,
    line_number: Option<u32>,
    title: Option<String>,
    body: String,
) {
    let now = chrono::Utc::now().to_rfc3339();
    let is_new_thread = thread_id.is_none();
    let thread_id = thread_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let result = (|| -> Result<(), String> {
        if is_new_thread {
            let title = title.unwrap_or_else(|| default_thread_title(&body));
            let anchor = if file_path.is_some() || line_number.is_some() {
                Some(ThreadAnchor {
                    file_path: file_path.clone(),
                    line_number,
                    side: None,
                    hunk_ref: None,
                    head_sha: None,
                })
            } else {
                None
            };

            let thread = Thread {
                id: thread_id.clone(),
                review_id: review_id.clone(),
                task_id: Some(task_id.clone()),
                title,
                status: ThreadStatus::Todo,
                impact: ThreadImpact::Nitpick,
                anchor,
                author: "User".to_string(),
                created_at: now.clone(),
                updated_at: now.clone(),
            };

            app.thread_repo
                .save(&thread)
                .map_err(|e| format!("Failed to save thread: {e}"))?;
        } else {
            app.thread_repo
                .touch(&thread_id)
                .map_err(|e| format!("Failed to update thread timestamp: {e}"))?;
        }

        let comment = Comment {
            id: uuid::Uuid::new_v4().to_string(),
            thread_id: thread_id.clone(),
            author: "User".to_string(),
            body,
            parent_id: None,
            created_at: now.clone(),
            updated_at: now,
        };

        app.comment_repo
            .save(&comment)
            .map_err(|e| format!("Failed to save comment: {e}"))?;

        Ok(())
    })();

    app.dispatch(Action::Async(AsyncAction::ThreadCommentSaved(
        result.clone(),
    )));

    if result.is_ok() {
        load_review_threads(app, review_id.clone());
        app.dispatch(Action::Review(ReviewAction::OpenThread {
            task_id,
            thread_id: Some(thread_id),
            file_path,
            line_number,
        }));
    }
}

fn update_thread_status(app: &mut LaReviewApp, thread_id: String, status: ThreadStatus) {
    let review_id = app.state.selected_review_id.clone();
    let result = app
        .thread_repo
        .update_status(&thread_id, status)
        .map(|_| ())
        .map_err(|e| format!("Failed to update thread status: {e}"));

    app.dispatch(Action::Async(AsyncAction::ThreadCommentSaved(
        result.clone(),
    )));

    if let (Ok(_), Some(review_id)) = (result, review_id) {
        load_review_threads(app, review_id);
    }
}

fn update_thread_impact(app: &mut LaReviewApp, thread_id: String, impact: ThreadImpact) {
    let review_id = app.state.selected_review_id.clone();
    let result = app
        .thread_repo
        .update_impact(&thread_id, impact)
        .map(|_| ())
        .map_err(|e| format!("Failed to update thread impact: {e}"));

    app.dispatch(Action::Async(AsyncAction::ThreadCommentSaved(
        result.clone(),
    )));

    if let (Ok(_), Some(review_id)) = (result, review_id) {
        load_review_threads(app, review_id);
    }
}

fn update_thread_title(app: &mut LaReviewApp, thread_id: String, title: String) {
    let review_id = app.state.selected_review_id.clone();
    let result = app
        .thread_repo
        .update_title(&thread_id, &title)
        .map(|_| ())
        .map_err(|e| format!("Failed to update thread title: {e}"));

    app.dispatch(Action::Async(AsyncAction::ThreadCommentSaved(
        result.clone(),
    )));

    if let (Ok(_), Some(review_id)) = (result, review_id) {
        load_review_threads(app, review_id);
    }
}

fn default_thread_title(body: &str) -> String {
    let first_line = body.lines().next().unwrap_or("").trim();
    if first_line.is_empty() {
        return "Untitled thread".to_string();
    }
    if first_line.len() > 80 {
        format!("{}...", &first_line[..77])
    } else {
        first_line.to_string()
    }
}

fn run_d2_command(app: &mut LaReviewApp, command: D2Command) {
    let command_str = match command {
        D2Command::Install => "curl -fsSL https://d2lang.com/install.sh | sh -s --",
        D2Command::Uninstall => "curl -fsSL https://d2lang.com/install.sh | sh -s -- --uninstall",
    }
    .to_string();

    let d2_install_tx = app.d2_install_tx.clone();

    crate::RUNTIME.get().unwrap().spawn(async move {
        let mut child = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command_str)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn D2 process");

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        use tokio::io::AsyncBufReadExt;
        let mut stdout_reader = tokio::io::BufReader::new(stdout).lines();
        let mut stderr_reader = tokio::io::BufReader::new(stderr).lines();

        loop {
            tokio::select! {
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => { let _ = d2_install_tx.send(line).await; }
                        _ => break,
                    }
                }
                line = stderr_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => { let _ = d2_install_tx.send(line).await; }
                        _ => break,
                    }
                }
            }
        }

        let _ = d2_install_tx
            .send("___INSTALL_COMPLETE___".to_string())
            .await;
    });
}

fn generate_export_preview(
    app: &mut LaReviewApp,
    review_id: crate::domain::ReviewId,
    run_id: crate::domain::ReviewRunId,
) {
    let review_repo = app.review_repo.clone();
    let run_repo = app.run_repo.clone();
    let task_repo = app.task_repo.clone();
    let thread_repo = app.thread_repo.clone();
    let comment_repo = app.comment_repo.clone();
    let action_tx = app.action_tx.clone();

    tokio::spawn(async move {
        let result = async {
            let review = review_repo
                .find_by_id(&review_id)
                .map_err(|e: anyhow::Error| e.to_string())?
                .ok_or("Review not found")?;
            let run = run_repo
                .find_by_id(&run_id)
                .map_err(|e: anyhow::Error| e.to_string())?
                .ok_or("Run not found")?;
            let tasks = task_repo
                .find_by_run(&run_id)
                .map_err(|e: anyhow::Error| e.to_string())?;

            let threads = thread_repo
                .find_by_review(&review_id)
                .map_err(|e: anyhow::Error| e.to_string())?;
            let mut comments = Vec::new();
            for thread in &threads {
                let mut thread_comments = comment_repo
                    .list_for_thread(&thread.id)
                    .map_err(|e: anyhow::Error| e.to_string())?;
                comments.append(&mut thread_comments);
            }

            let data = crate::application::review::export::ExportData {
                review,
                run,
                tasks,
                threads,
                comments,
            };

            crate::application::review::export::ReviewExporter::export_to_markdown(&data, true)
                .await
                .map_err(|e| e.to_string())
        }
        .await;

        let _ = action_tx
            .send(Action::Async(AsyncAction::ExportPreviewGenerated(result)))
            .await;
    });
}

fn export_review(
    app: &mut LaReviewApp,
    review_id: crate::domain::ReviewId,
    run_id: crate::domain::ReviewRunId,
    path: std::path::PathBuf,
) {
    let review_repo = app.review_repo.clone();
    let run_repo = app.run_repo.clone();
    let task_repo = app.task_repo.clone();
    let thread_repo = app.thread_repo.clone();
    let comment_repo = app.comment_repo.clone();
    let action_tx = app.action_tx.clone();

    tokio::spawn(async move {
        let result: anyhow::Result<()> = async {
            let review = review_repo
                .find_by_id(&review_id)
                .map_err(|e: anyhow::Error| anyhow::anyhow!(e))?
                .ok_or_else(|| anyhow::anyhow!("Review not found"))?;
            let run = run_repo
                .find_by_id(&run_id)
                .map_err(|e: anyhow::Error| anyhow::anyhow!(e))?
                .ok_or_else(|| anyhow::anyhow!("Run not found"))?;
            let tasks = task_repo
                .find_by_run(&run_id)
                .map_err(|e: anyhow::Error| anyhow::anyhow!(e))?;

            let threads = thread_repo
                .find_by_review(&review_id)
                .map_err(|e: anyhow::Error| anyhow::anyhow!(e))?;
            let mut comments = Vec::new();
            for thread in &threads {
                let mut thread_comments = comment_repo
                    .list_for_thread(&thread.id)
                    .map_err(|e: anyhow::Error| anyhow::anyhow!(e))?;
                comments.append(&mut thread_comments);
            }

            let data = crate::application::review::export::ExportData {
                review,
                run,
                tasks,
                threads,
                comments,
            };

            let export_result =
                crate::application::review::export::ReviewExporter::export_to_markdown(
                    &data, false,
                )
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            // Write Markdown
            std::fs::write(&path, &export_result.markdown)?;

            // Write Assets
            if !export_result.assets.is_empty() {
                let parent_dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
                let assets_dir = parent_dir.join("assets");
                std::fs::create_dir_all(&assets_dir)?;

                for (filename, bytes) in export_result.assets {
                    std::fs::write(assets_dir.join(filename), bytes)?;
                }
            }

            Ok(())
        }
        .await;

        let _ = action_tx
            .send(Action::Async(AsyncAction::ExportFinished(
                result.map_err(|e: anyhow::Error| e.to_string()),
            )))
            .await;
    });
}

fn save_repo(app: &mut LaReviewApp, repo: crate::domain::LinkedRepo) {
    let repo_repo = app.repo_repo.clone();
    let action_tx = app.action_tx.clone();

    tokio::spawn(async move {
        let result = repo_repo
            .save(&repo)
            .map(|_| repo.clone())
            .map_err(|e| e.to_string());
        let _ = action_tx
            .send(Action::Async(AsyncAction::RepoSaved(result)))
            .await;
    });
}

fn delete_repo(app: &mut LaReviewApp, repo_id: String) {
    let repo_repo = app.repo_repo.clone();
    let action_tx = app.action_tx.clone();

    tokio::spawn(async move {
        let result = repo_repo
            .delete(&repo_id)
            .map(|_| repo_id.clone())
            .map_err(|e| e.to_string());
        let _ = action_tx
            .send(Action::Async(AsyncAction::RepoDeleted(result)))
            .await;
    });
}

fn pick_folder_for_link(app: &mut LaReviewApp) {
    let action_tx = app.action_tx.clone();

    tokio::spawn(async move {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            let remotes = crate::infra::git::extract_git_remotes(&path);

            let repo = crate::domain::LinkedRepo {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                path,
                remotes,
                created_at: chrono::Utc::now().to_rfc3339(),
            };

            let _ = action_tx
                .send(Action::Async(AsyncAction::NewRepoPicked(repo)))
                .await;
        }
    });
}
