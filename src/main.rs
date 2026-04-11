#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use anyhow::Result;
use clap::Parser;
use log::{error, info};

use lareview::infra;
use lareview::infra::cli::args::{process_cli_args, CliArgs};
use lareview::infra::cli::diff::try_read_stdin_diff;
use lareview::state::{AppState, DiffRequest, PendingDiff};
use tauri::{Emitter, Manager};

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
    match CliArgs::try_parse() {
        Ok(parsed_args) => {
            debug_log(&format!(
                "Parsed initial args command: {:?}",
                parsed_args.command
            ));

            // Read piped stdin once, up front. `try_read_stdin_diff` returns
            // `None` when stdin is a terminal, so this is a no-op during a
            // normal GUI launch.
            let piped_stdin = try_read_stdin_diff().unwrap_or(None);

            // Process CLI args into data structures, WITHOUT touching DB/AppState yet.
            let (initial_req, initial_pending) = process_cli_args(&parsed_args, piped_stdin)?;

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

fn run_gui(initial_req: Option<DiffRequest>, initial_pending: Option<PendingDiff>) -> Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            debug_log(&format!(
                "Single Instance Callback triggered! Argv: {:?}",
                argv
            ));

            match CliArgs::try_parse_from(&argv) {
                Ok(args) => {
                    debug_log(&format!(
                        "Successfully parsed second instance args: {:?}",
                        args.command
                    ));
                    // Second-instance callback runs in the already-launched
                    // GUI process; its stdin is not connected to the new
                    // CLI invocation, so we cannot read the piped diff here.
                    // Pass `None` and rely on explicit CLI args.
                    match process_cli_args(&args, None) {
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
            lareview::commands::add_custom_agent,
            lareview::commands::delete_custom_agent,
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
            lareview::commands::get_feedback_filter_config,
            lareview::commands::update_feedback_filter_config,
            lareview::commands::get_timeout_config,
            lareview::commands::update_timeout_config,
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
            // Merge confidence
            lareview::commands::get_merge_confidence,
            // Rule library
            lareview::commands::get_rule_library,
            lareview::commands::get_rule_library_by_category,
            lareview::commands::add_rule_from_library,
            lareview::commands::get_default_issue_categories,
            // Rule analytics
            lareview::commands::get_rule_rejection_stats,
            // Learning system
            lareview::commands::get_learned_patterns,
            lareview::commands::create_learned_pattern,
            lareview::commands::update_learned_pattern,
            lareview::commands::delete_learned_pattern,
            lareview::commands::toggle_learned_pattern,
            lareview::commands::get_learning_status,
            lareview::commands::trigger_learning_compaction,
        ])
        .run(tauri::generate_context!())
        .map_err(|e| anyhow::anyhow!("{}", e))
}
