#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use log::{error, info};

use lareview::infra;
use lareview::infra::cli::diff::{self, read_stdin_diff};
use lareview::infra::cli::repo::detect_git_repo;
use lareview::state::{AppState, DiffRequest, PendingDiff};
use tauri::{Emitter, Manager};

#[derive(Parser, Debug)]
#[command(name = "lareview")]
#[command(author = "LaReview Team")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "LaReview - Better Code Reviews", long_about = None)]
struct Args {
    /// Agent to use for review (claude, codex, qwen, etc.)
    #[arg(short, long)]
    agent: Option<String>,

    /// Branch, tag, or commit to diff from
    #[arg()]
    from: Option<String>,

    /// Branch, tag, or commit to diff to (requires --from)
    #[arg()]
    to: Option<String>,

    /// PR reference (owner/repo#number or URL)
    #[arg(short, long)]
    pr: Option<String>,

    /// Review uncommitted changes
    #[arg(long)]
    status: bool,

    /// Open with pre-loaded diff from stdin
    #[arg(long)]
    stdin: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Open the GUI (default behavior)
    Gui,

    /// Review changes between branches/tags/commits
    Diff {
        /// Source ref
        #[arg(index = 1)]
        from: String,
        /// Target ref
        #[arg(index = 2)]
        to: String,
    },

    /// Review a GitHub PR
    Pr {
        /// PR reference (owner/repo#number or URL)
        #[arg(index = 1)]
        pr_ref: String,
    },

    /// Review uncommitted changes
    Status,

    /// Review git stash entries
    Stash {
        /// Stash index (default: 0, latest)
        #[arg(default_value = "0")]
        index: usize,
    },
}

use std::io::Write;

fn debug_log(msg: &str) {
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/lareview_debug.log")
    {
        let _ = writeln!(file, "[{}] {}", chrono::Utc::now().to_rfc3339(), msg);
    }
}

fn main() -> Result<()> {
    debug_log("Application starting");
    let _ = fix_path_env::fix();
    let _ = env_logger::try_init();

    let args: Vec<String> = std::env::args().collect();
    debug_log(&format!("Raw args: {:?}", args));

    let is_mcp_server = args.contains(&"--task-mcp-server".to_string())
        || std::env::var("MCP_TRANSPORT").is_ok()
        || std::env::var("MCP_SERVER_NAME").is_ok();

    if is_mcp_server {
        info!("Starting MCP task server...");
        lareview::block_on(async {
            if let Err(e) = infra::acp::run_task_mcp_server().await {
                error!("MCP server error: {e}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }

    // Try parsing args for this instance (primary launch or CLI tool)
    match Args::try_parse() {
        Ok(parsed_args) => {
            debug_log(&format!(
                "Parsed initial args command: {:?}",
                parsed_args.command
            ));

            // Process CLI args into data structures, WITHOUT touching DB/AppState yet.
            let (initial_req, initial_pending) = process_cli_args(&parsed_args)?;

            // Run GUI. We pass the initial data.
            // AppState will be created INSIDE run_gui setup, ensuring Second Instance never touches DB.
            run_gui(initial_req, initial_pending)
        }
        Err(e) => {
            debug_log(&format!("Arg parse error: {}", e));
            e.print().expect("failed to print help");
            Ok(())
        }
    }
}

fn process_cli_args(args: &Args) -> Result<(Option<DiffRequest>, Option<PendingDiff>)> {
    debug_log("process_cli_args called");
    let mut diff_req = None;
    let mut pending = None;

    if let Some(cmd) = &args.command {
        debug_log(&format!("Processing command: {:?}", cmd));
        match cmd {
            Commands::Gui => {}
            Commands::Diff { from, to } => {
                debug_log(&format!("Diff command: {} .. {}", from, to));
                diff_req = Some(DiffRequest {
                    from: from.clone(),
                    to: to.clone(),
                    agent: args.agent.clone(),
                    source: format!("git diff {}..{}", from, to),
                });
            }
            Commands::Pr { pr_ref } => {
                debug_log(&format!("PR command: {}", pr_ref));
                let remote_ref = diff::parse_remote_ref(pr_ref)?;
                match remote_ref {
                    diff::RemoteRef::GitHub {
                        owner,
                        repo,
                        number,
                    } => {
                        debug_log(&format!("Parsed PR: {}/{}#{}", owner, repo, number));
                        diff_req = Some(DiffRequest {
                            from: format!("{}/{}/pull/{}", owner, repo, number),
                            to: String::new(),
                            agent: args.agent.clone(),
                            source: format!("PR {}", pr_ref),
                        });
                    }
                    diff::RemoteRef::GitLab {
                        host,
                        project_path,
                        number,
                    } => {
                        debug_log(&format!("Parsed MR: {}/{}!{}", host, project_path, number));
                        diff_req = Some(DiffRequest {
                            from: format!(
                                "https://{host}/{project_path}/-/merge_requests/{number}"
                            ),
                            to: String::new(),
                            agent: args.agent.clone(),
                            source: format!("MR {}", pr_ref),
                        });
                    }
                }
            }
            Commands::Status => {
                diff_req = Some(DiffRequest {
                    from: String::new(),
                    to: String::new(),
                    agent: args.agent.clone(),
                    source: "uncommitted changes".to_string(),
                });
            }
            Commands::Stash { index } => {
                let diff = diff::get_stash_diff(*index)?;
                pending = Some(PendingDiff {
                    diff,
                    repo_root: detect_git_repo(),
                    agent: args.agent.clone(),
                    source: format!("stash@{{{}}}", index),
                    created_at: chrono::Utc::now(),
                });
            }
        }
    } else if let Some(pr_ref) = &args.pr {
        let remote_ref = diff::parse_remote_ref(pr_ref)?;
        match remote_ref {
            diff::RemoteRef::GitHub {
                owner,
                repo,
                number,
            } => {
                diff_req = Some(DiffRequest {
                    from: format!("{}/{}/pull/{}", owner, repo, number),
                    to: String::new(),
                    agent: args.agent.clone(),
                    source: format!("PR {}", pr_ref),
                });
            }
            diff::RemoteRef::GitLab {
                host,
                project_path,
                number,
            } => {
                diff_req = Some(DiffRequest {
                    from: format!("https://{host}/{project_path}/-/merge_requests/{number}"),
                    to: String::new(),
                    agent: args.agent.clone(),
                    source: format!("MR {}", pr_ref),
                });
            }
        }
    } else if args.status {
        diff_req = Some(DiffRequest {
            from: String::new(),
            to: String::new(),
            agent: args.agent.clone(),
            source: "uncommitted changes".to_string(),
        });
    } else if let (Some(from), Some(to)) = (&args.from, &args.to) {
        diff_req = Some(DiffRequest {
            from: from.clone(),
            to: to.clone(),
            agent: args.agent.clone(),
            source: format!("git diff {}..{}", from, to),
        });
    } else if let Some(from) = &args.from {
        diff_req = Some(DiffRequest {
            from: from.clone(),
            to: "HEAD".to_string(),
            agent: args.agent.clone(),
            source: format!("git diff {}..HEAD", from),
        });
    } else if args.stdin {
        let diff = read_stdin_diff().context("Failed to read diff from stdin")?;
        if diff.trim().is_empty() {
            anyhow::bail!("Error: No diff provided via stdin");
        }
        pending = Some(PendingDiff {
            diff,
            repo_root: detect_git_repo(),
            agent: args.agent.clone(),
            source: "stdin".to_string(),
            created_at: chrono::Utc::now(),
        });
    }

    Ok((diff_req, pending))
}

fn run_gui(initial_req: Option<DiffRequest>, initial_pending: Option<PendingDiff>) -> Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            debug_log(&format!(
                "Single Instance Callback triggered! Argv: {:?}",
                argv
            ));

            match Args::try_parse_from(&argv) {
                Ok(args) => {
                    debug_log(&format!(
                        "Successfully parsed second instance args: {:?}",
                        args.command
                    ));
                    // We process args again, here inside the Primary instance.
                    match process_cli_args(&args) {
                        Ok((req, pending)) => {
                            let state = app.state::<AppState>();

                            if let Some(r) = req {
                                debug_log("Updating diff_request from callback");
                                *state.diff_request.lock().unwrap() = Some(r);
                            }
                            if let Some(p) = pending {
                                debug_log("Updating pending_diff from callback");
                                *state.pending_diff.lock().unwrap() = Some(p);
                            }

                            if let Some(window) = app.get_webview_window("main") {
                                debug_log("Focusing main window and emitting diff-ready");
                                let _ = window.set_focus();
                                let _ = window.emit("lareview:diff-ready", ());
                            } else {
                                debug_log("Main window not found!");
                            }
                        }
                        Err(e) => {
                            debug_log(&format!(
                                "Failed to handle CLI args from second instance: {}",
                                e
                            ));
                            error!("Failed to handle CLI args from second instance: {}", e);
                        }
                    }
                }
                Err(e) => {
                    debug_log(&format!("Failed to parse args from second instance: {}", e));
                    error!("Failed to parse args from second instance: {}", e);
                }
            }
        }))
        .setup(move |app| {
            // Initialize AppState HERE (only for Primary instance)
            debug_log("Initializing AppState (Primary Instance)...");
            let app_state = AppState::new();

            // Apply initial args
            if let Some(r) = initial_req {
                *app_state.diff_request.lock().unwrap() = Some(r);
            }
            if let Some(p) = initial_pending {
                *app_state.pending_diff.lock().unwrap() = Some(p);
            }

            app.manage(app_state);
            debug_log("AppState initialized and managed.");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            lareview::commands::get_app_version,
            lareview::commands::get_cli_status,
            lareview::commands::install_cli,
            lareview::commands::get_pending_reviews,
            lareview::commands::get_all_reviews,
            lareview::commands::get_review_runs,
            lareview::commands::get_linked_repos,
            lareview::commands::parse_diff,
            lareview::commands::get_file_content,
            lareview::commands::generate_review,
            lareview::commands::load_tasks,
            lareview::commands::update_task_status,
            lareview::commands::save_feedback,
            lareview::commands::get_feedback_by_review,
            lareview::commands::get_feedback_diff_snippet,
            lareview::commands::get_feedback_comments,
            lareview::commands::add_comment,
            lareview::commands::update_feedback_status,
            lareview::commands::update_feedback_impact,
            lareview::commands::delete_feedback,
            lareview::commands::export_review,
            lareview::commands::fetch_remote_pr,
            lareview::commands::get_agents,
            lareview::commands::update_agent_config,
            lareview::commands::get_github_token,
            lareview::commands::set_github_token,
            lareview::commands::get_vcs_status,
            lareview::commands::get_single_vcs_status,
            lareview::commands::link_repo,
            lareview::commands::clone_and_link_repo,
            lareview::commands::unlink_repo,
            lareview::commands::delete_review,
            lareview::commands::get_available_editors,
            lareview::commands::get_editor_config,
            lareview::commands::update_editor_config,
            lareview::commands::get_review_rules,
            lareview::commands::create_review_rule,
            lareview::commands::update_review_rule,
            lareview::commands::delete_review_rule,
            lareview::commands::open_in_editor,
            lareview::commands::get_repo_root_for_review,
            lareview::commands::get_cli_status,
            lareview::commands::install_cli,
            lareview::commands::get_pending_review_from_state,
            lareview::commands::copy_to_clipboard,
            lareview::commands::open_url,
            lareview::commands::clear_pending_diff,
            lareview::commands::get_diff_request,
            lareview::commands::acquire_diff_from_request,
            lareview::commands::push_remote_review,
            lareview::commands::export_review_markdown,
            lareview::commands::push_remote_feedback,
            lareview::commands::stop_generation,
            lareview::commands::set_repo_snapshot_access,
            // Issue checks
            lareview::commands::get_issue_checks_for_run,
            // Rule library
            lareview::commands::get_rule_library,
            lareview::commands::get_rule_library_by_category,
            lareview::commands::add_rule_from_library,
            lareview::commands::get_default_issue_categories,
        ])
        .run(tauri::generate_context!())
        .map_err(|e| anyhow::anyhow!("{}", e))
}
