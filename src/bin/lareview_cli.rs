//! LaReview CLI entry point.
//!
//! Provides a terminal interface to launch the GUI with pre-loaded diffs.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::Path;
use std::process::{Command, Stdio};

use lareview::infra::cli::diff::{self, DiffSource, read_stdin_diff, try_read_stdin_diff};
use lareview::infra::cli::pending::{PendingReview, save_pending_review};
use lareview::infra::cli::repo::detect_git_repo;

#[derive(Parser, Debug)]
#[command(name = "lareview")]
#[command(author = "LaReview Team")]
#[command(version = "0.0.16")]
#[command(about = "AI-powered code review companion", long_about = None)]
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
    /// Review changes between branches/tags/commits
    Diff {
        /// Source ref
        from: String,
        /// Target ref
        to: String,
    },

    /// Review a GitHub PR
    Pr {
        /// PR reference (owner/repo#number or URL)
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Handle subcommands
    let (diff, source_desc) = if let Some(cmd) = args.command {
        match cmd {
            Commands::Diff { from, to } => {
                let diff = diff::acquire_diff(DiffSource::GitDiff {
                    from: from.clone(),
                    to: to.clone(),
                })?;
                (diff, format!("git diff {}..{}", from, to))
            }
            Commands::Pr { pr_ref } => {
                let (owner, repo, number) = diff::parse_pr_ref(&pr_ref)?;
                let diff = diff::acquire_diff(DiffSource::GitHubPr {
                    owner,
                    repo,
                    number,
                })?;
                (diff, format!("PR {}", pr_ref))
            }
            Commands::Status => {
                let diff = diff::acquire_diff(DiffSource::GitStatus)?;
                (diff, "uncommitted changes".to_string())
            }
            Commands::Stash { index } => {
                let diff = diff::get_stash_diff(index)?;
                (diff, format!("stash@{{{}}}", index))
            }
        }
    } else if let Some(pr_ref) = args.pr {
        let (owner, repo, number) = diff::parse_pr_ref(&pr_ref)?;
        let diff = diff::acquire_diff(DiffSource::GitHubPr {
            owner,
            repo,
            number,
        })?;
        (diff, format!("PR {}", pr_ref))
    } else if args.status {
        let diff = diff::acquire_diff(DiffSource::GitStatus)?;
        (diff, "uncommitted changes".to_string())
    } else if args.from.is_some() && args.to.is_some() {
        let from = args.from.unwrap();
        let to = args.to.unwrap();
        let diff = diff::acquire_diff(DiffSource::GitDiff {
            from: from.clone(),
            to: to.clone(),
        })?;
        (diff, format!("git diff {}..{}", from, to))
    } else if let Some(from) = args.from {
        let diff = diff::acquire_diff(DiffSource::GitDiff {
            from: from.clone(),
            to: "HEAD".to_string(),
        })?;
        (diff, format!("git diff {}..HEAD", from))
    } else if args.stdin {
        let diff = read_stdin_diff()?;
        if diff.trim().is_empty() {
            eprintln!("Error: No diff provided via stdin");
            std::process::exit(1);
        }
        (diff, "stdin".to_string())
    } else if let Some(stdin_diff) = try_read_stdin_diff()? {
        (stdin_diff, "stdin".to_string())
    } else {
        (String::new(), "no diff".to_string())
    };

    // Detect git repo
    let repo_root = detect_git_repo();

    // Build pending review
    let pending = PendingReview {
        diff,
        repo_root,
        agent: args.agent,
        auto_generate: false,
        source: source_desc,
        created_at: chrono::Utc::now(),
    };

    // Save pending review
    let pending_path = save_pending_review(&pending).context("Failed to save pending review")?;

    // Launch GUI
    launch_gui(&pending_path)?;

    Ok(())
}

fn launch_gui(pending_path: &Path) -> Result<()> {
    // Get path to our GUI binary
    let gui_binary = std::env::current_exe()
        .context("Failed to get current executable")?
        .with_file_name("lareview-gui");

    // Check if GUI binary exists
    if !gui_binary.exists() {
        // Try the current process (we might be the GUI binary invoked as CLI)
        let current_exe = std::env::current_exe().context("Failed to get current executable")?;

        // If the current binary name suggests it's the GUI, just relaunch with flag
        if current_exe
            .file_name()
            .map(|n| n.to_string_lossy().contains("gui"))
            .unwrap_or(false)
        {
            // We're the GUI binary, relaunch ourselves with --open-pending
            let status = Command::new(&current_exe)
                .args(["--open-pending", &pending_path.to_string_lossy()])
                .spawn()
                .context("Failed to launch GUI")?
                .wait()
                .context("GUI exited with error")?;

            if !status.success() {
                anyhow::bail!("GUI exited with non-zero status: {}", status);
            }
            return Ok(());
        }

        // Try running via cargo for development
        let status = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "lareview-gui",
                "--",
                "--open-pending",
                &pending_path.to_string_lossy(),
            ])
            .stdin(Stdio::null())
            .spawn()
            .context("Failed to launch GUI via cargo")?
            .wait()
            .context("GUI exited with error")?;

        if !status.success() {
            anyhow::bail!("GUI exited with non-zero status: {}", status);
        }
        return Ok(());
    }

    // Spawn GUI binary
    let status = Command::new(&gui_binary)
        .args(["--open-pending", &pending_path.to_string_lossy()])
        .stdin(Stdio::null())
        .spawn()
        .context("Failed to launch GUI")?
        .wait()
        .context("GUI exited with error")?;

    if !status.success() {
        anyhow::bail!("GUI exited with non-zero status: {}", status);
    }

    Ok(())
}
