//! CLI argument parsing and dispatch to initial app state.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use crate::infra::cli::diff::{self, get_stash_diff};
use crate::infra::cli::repo::detect_git_repo;
use crate::state::{DiffRequest, PendingDiff};

#[derive(Parser, Debug, Clone)]
#[command(name = "lareview")]
#[command(author = "LaReview Team")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "LaReview - Better Code Reviews", long_about = None)]
pub struct CliArgs {
    /// Agent to use for review (claude, codex, qwen, etc.)
    #[arg(short, long)]
    pub agent: Option<String>,

    /// Branch, tag, or commit to diff from
    #[arg()]
    pub from: Option<String>,

    /// Branch, tag, or commit to diff to (requires --from)
    #[arg()]
    pub to: Option<String>,

    /// PR reference (owner/repo#number or URL)
    #[arg(short, long)]
    pub pr: Option<String>,

    /// Review uncommitted changes
    #[arg(long)]
    pub status: bool,

    /// Open with pre-loaded diff from stdin
    #[arg(long)]
    pub stdin: bool,

    #[command(subcommand)]
    pub command: Option<CliCommand>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum CliCommand {
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

/// Translate parsed CLI args into initial app state.
///
/// `piped_stdin` should contain the stdin contents when the process was
/// invoked with a pipe (e.g. `git diff | lareview ...`). It is `None` when
/// stdin is a terminal. This keeps the function pure and unit-testable.
pub fn process_cli_args(
    args: &CliArgs,
    piped_stdin: Option<String>,
) -> Result<(Option<DiffRequest>, Option<PendingDiff>)> {
    let mut diff_req = None;
    let mut pending = None;

    if let Some(cmd) = &args.command {
        match cmd {
            CliCommand::Gui => {}
            CliCommand::Diff { from, to } => {
                diff_req = Some(DiffRequest {
                    from: from.clone(),
                    to: to.clone(),
                    agent: args.agent.clone(),
                    source: format!("git diff {}..{}", from, to),
                });
            }
            CliCommand::Pr { pr_ref } => {
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
            CliCommand::Status => {
                diff_req = Some(DiffRequest {
                    from: String::new(),
                    to: String::new(),
                    agent: args.agent.clone(),
                    source: "uncommitted changes".to_string(),
                });
            }
            CliCommand::Stash { index } => {
                let diff = get_stash_diff(*index)?;
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
        let diff = piped_stdin
            .clone()
            .context("--stdin was specified but stdin was not piped")?;
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

    // Fallback: if nothing above matched but stdin was piped with content,
    // treat it as a pending diff. This supports the ergonomic invocation
    // `git diff main | lareview --agent claude` without requiring --stdin.
    if let (None, None, Some(diff)) = (
        &diff_req,
        &pending,
        piped_stdin.filter(|d| !d.trim().is_empty()),
    ) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn base_args() -> CliArgs {
        CliArgs {
            agent: None,
            from: None,
            to: None,
            pr: None,
            status: false,
            stdin: false,
            command: None,
        }
    }

    #[test]
    fn explicit_stdin_flag_with_piped_diff_creates_pending() {
        let args = CliArgs {
            stdin: true,
            agent: Some("claude".to_string()),
            ..base_args()
        };
        let diff = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@\n-a\n+b\n".to_string();

        let (req, pending) = process_cli_args(&args, Some(diff.clone())).unwrap();

        assert!(req.is_none());
        let p = pending.expect("expected PendingDiff");
        assert_eq!(p.diff, diff);
        assert_eq!(p.agent.as_deref(), Some("claude"));
        assert_eq!(p.source, "stdin");
    }

    #[test]
    fn piped_diff_without_stdin_flag_still_creates_pending() {
        // Repro for issue #13: `git diff develop | lareview --agent claude`
        // does nothing because `--stdin` is not passed. It should still
        // auto-detect the piped diff and start a review.
        let args = CliArgs {
            agent: Some("claude".to_string()),
            ..base_args()
        };
        let diff = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@\n-a\n+b\n".to_string();

        let (req, pending) = process_cli_args(&args, Some(diff.clone())).unwrap();

        assert!(req.is_none(), "no diff request expected");
        let p = pending.expect(
            "expected PendingDiff when stdin is piped — issue #13 \
             reproduces if this is None",
        );
        assert_eq!(p.diff, diff);
        assert_eq!(p.agent.as_deref(), Some("claude"));
        assert_eq!(p.source, "stdin");
    }

    #[test]
    fn no_args_and_no_pipe_produces_nothing() {
        let args = base_args();
        let (req, pending) = process_cli_args(&args, None).unwrap();
        assert!(req.is_none());
        assert!(pending.is_none());
    }

    #[test]
    fn explicit_refs_ignore_piped_stdin() {
        // Explicit CLI args win — stdin is ignored when user is clearly
        // asking for a specific diff.
        let args = CliArgs {
            from: Some("main".to_string()),
            to: Some("feature".to_string()),
            agent: Some("claude".to_string()),
            ..base_args()
        };
        let (req, pending) =
            process_cli_args(&args, Some("should be ignored".to_string())).unwrap();
        assert!(pending.is_none());
        let r = req.unwrap();
        assert_eq!(r.from, "main");
        assert_eq!(r.to, "feature");
    }
}
