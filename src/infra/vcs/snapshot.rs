//! Git snapshot operations for temporary commit checkouts.
//!
//! Provides a manager for creating and removing lightweight snapshots of a commit.
//! Used to give agents access to PR code at a specific commit without
//! affecting the user's working directory.

use anyhow::{Context, Result, bail};
use std::path::{Component, Path, PathBuf};
use tokio::process::Command;

/// Base directory name for LaReview snapshots in temp directory.
const SNAPSHOT_DIR_PREFIX: &str = "lareview-snapshots";

/// Manages git snapshots for a repository.
#[derive(Debug, Clone)]
pub struct SnapshotManager {
    /// Path to the main repository.
    repo_path: PathBuf,
}

impl SnapshotManager {
    /// Create a new snapshot manager for the given repository.
    pub fn new(repo_path: impl Into<PathBuf>) -> Self {
        Self {
            repo_path: repo_path.into(),
        }
    }

    /// Get the base directory for snapshots in the temp directory.
    fn snapshot_base_dir() -> PathBuf {
        std::env::temp_dir().join(SNAPSHOT_DIR_PREFIX)
    }

    fn git_command(&self) -> Command {
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(&self.repo_path)
            .env("LC_ALL", "C")
            .kill_on_drop(true);
        command
    }

    fn git_command_blocking(&self) -> std::process::Command {
        let mut command = std::process::Command::new("git");
        command.arg("-C").arg(&self.repo_path).env("LC_ALL", "C");
        command
    }

    fn validate_session_id(session_id: &str) -> Result<()> {
        let mut components = Path::new(session_id).components();
        if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
            bail!("snapshot session ID must be a single path component");
        }
        Ok(())
    }

    fn validate_object_id(object_id: &str) -> Result<()> {
        if !matches!(object_id.len(), 40 | 64)
            || !object_id.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            bail!("invalid Git object ID: {object_id}");
        }
        Ok(())
    }

    async fn commit_exists(&self, commit_sha: &str) -> Result<bool> {
        let revision = format!("{commit_sha}^{{commit}}");
        let output = self
            .git_command()
            .args(["cat-file", "-e", &revision])
            // A presence check must not trigger a network request in a partial clone.
            .env("GIT_NO_LAZY_FETCH", "1")
            .output()
            .await
            .context("check whether snapshot commit exists")?;
        Ok(output.status.success())
    }

    async fn snapshot_remote(&self) -> Result<String> {
        let partial_clone = self
            .git_command()
            .args(["config", "--get", "extensions.partialClone"])
            .output()
            .await
            .context("read partial-clone remote")?;
        if partial_clone.status.success() {
            let remote = String::from_utf8_lossy(&partial_clone.stdout)
                .trim()
                .to_string();
            if !remote.is_empty() {
                return Ok(remote);
            }
        }

        let promisors = self
            .git_command()
            .args(["config", "--get-regexp", r"^remote\..*\.promisor$"])
            .output()
            .await
            .context("list promisor remotes")?;
        if promisors.status.success() {
            for line in String::from_utf8_lossy(&promisors.stdout).lines() {
                let Some((key, value)) = line.split_once(char::is_whitespace) else {
                    continue;
                };
                if !value.trim().eq_ignore_ascii_case("true") {
                    continue;
                }
                if let Some(remote) = key
                    .strip_prefix("remote.")
                    .and_then(|key| key.strip_suffix(".promisor"))
                    && !remote.is_empty()
                {
                    return Ok(remote.to_string());
                }
            }
        }

        let remotes = self
            .git_command()
            .arg("remote")
            .output()
            .await
            .context("list Git remotes")?;
        if remotes.status.success() {
            let stdout = String::from_utf8_lossy(&remotes.stdout);
            let remotes: Vec<&str> = stdout.lines().filter(|remote| !remote.is_empty()).collect();
            if remotes.contains(&"origin") {
                return Ok("origin".to_string());
            }
            if let Some(remote) = remotes.first() {
                return Ok((*remote).to_string());
            }
        }

        bail!("snapshot commit is missing and the repository has no Git remote")
    }

    async fn fetch(&self, remote: &str, object_id: Option<&str>, refetch: bool) -> Result<()> {
        let mut command = self.git_command();
        command.arg("fetch");
        if refetch {
            command.arg("--refetch");
        }
        command.args(["--no-auto-maintenance", "--no-write-commit-graph"]);
        command.arg(remote);
        if let Some(object_id) = object_id {
            command.arg(object_id);
        }

        let output = command.output().await.context("run git fetch")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            bail!("git fetch from {remote} failed: {stderr}");
        }
        Ok(())
    }

    async fn ensure_commit(&self, commit_sha: &str) -> Result<()> {
        if self.commit_exists(commit_sha).await? {
            return Ok(());
        }

        let remote = self.snapshot_remote().await?;
        match self.fetch(&remote, Some(commit_sha), false).await {
            Ok(()) => {}
            Err(error) => {
                let message = error.to_string();
                if let Some(missing_oid) = missing_commit_graph_object(&message) {
                    self.fetch(&remote, Some(&missing_oid), true)
                        .await
                        .with_context(|| {
                            format!("repair missing commit-graph object {missing_oid}")
                        })?;
                    self.fetch(&remote, Some(commit_sha), false).await?;
                } else {
                    // A raw PR commit may be rejected by the server but arrive through the
                    // repository's configured refspecs.
                    self.fetch(&remote, None, false)
                        .await
                        .with_context(|| message.clone())?;
                }
            }
        }

        if !self.commit_exists(commit_sha).await? {
            bail!("Git fetch completed but snapshot commit {commit_sha} is still missing");
        }
        Ok(())
    }

    async fn add_worktree(
        &self,
        snapshot_path: &Path,
        commit_sha: &str,
    ) -> Result<std::process::Output> {
        self.git_command()
            .args(["worktree", "add", "--detach", "--quiet"])
            .arg(snapshot_path)
            .arg(commit_sha)
            .output()
            .await
            .context("run git worktree add")
    }

    async fn cleanup_failed_worktree(&self, snapshot_path: &Path) {
        if let Err(error) = self.remove(snapshot_path).await {
            log::warn!(
                "Failed to clean up incomplete snapshot {}: {error}",
                snapshot_path.display()
            );
        }
    }

    /// Create a snapshot of the specified commit.
    ///
    /// Returns the path to the created snapshot directory.
    pub async fn create(&self, session_id: &str, commit_sha: &str) -> Result<PathBuf> {
        Self::validate_session_id(session_id)?;
        Self::validate_object_id(commit_sha)?;

        let base_dir = Self::snapshot_base_dir();
        std::fs::create_dir_all(&base_dir)
            .with_context(|| format!("create snapshot base dir: {}", base_dir.display()))?;

        let snapshot_path = base_dir.join(session_id);
        if snapshot_path.exists() {
            self.remove(&snapshot_path).await.with_context(|| {
                format!("remove existing snapshot: {}", snapshot_path.display())
            })?;
        }

        self.ensure_commit(commit_sha).await?;

        let checkout = self.add_worktree(&snapshot_path, commit_sha).await?;
        if checkout.status.success() {
            return Ok(snapshot_path);
        }

        let first_error = String::from_utf8_lossy(&checkout.stderr).trim().to_string();
        self.cleanup_failed_worktree(&snapshot_path).await;

        let Some(missing_oid) = missing_commit_graph_object(&first_error) else {
            bail!("git worktree add failed: {first_error}");
        };

        let remote = self.snapshot_remote().await?;
        self.fetch(&remote, Some(&missing_oid), true)
            .await
            .with_context(|| format!("repair missing commit-graph object {missing_oid}"))?;

        let retry = self.add_worktree(&snapshot_path, commit_sha).await?;
        if retry.status.success() {
            return Ok(snapshot_path);
        }

        let retry_error = String::from_utf8_lossy(&retry.stderr).trim().to_string();
        self.cleanup_failed_worktree(&snapshot_path).await;
        bail!(
            "git worktree add failed after repairing {missing_oid}: {retry_error} (initial error: {first_error})"
        )
    }

    /// Remove a snapshot and its linked-worktree metadata.
    pub async fn remove(&self, snapshot_path: &Path) -> Result<()> {
        let output = self
            .git_command()
            .args(["worktree", "remove", "--force"])
            .arg(snapshot_path)
            .output()
            .await
            .context("run git worktree remove")?;

        if output.status.success() {
            log::info!("Cleaned up snapshot at {}", snapshot_path.display());
            return Ok(());
        }

        // An interrupted `worktree add` may leave files without a valid registration.
        if snapshot_path.exists() {
            std::fs::remove_dir_all(snapshot_path).with_context(|| {
                format!(
                    "remove incomplete snapshot dir: {}",
                    snapshot_path.display()
                )
            })?;
        }
        let _ = self
            .git_command()
            .args(["worktree", "prune"])
            .output()
            .await;
        Ok(())
    }

    /// Best-effort synchronous cleanup for scope guards.
    pub fn remove_blocking(&self, snapshot_path: &Path) -> Result<()> {
        let output = self
            .git_command_blocking()
            .args(["worktree", "remove", "--force"])
            .arg(snapshot_path)
            .output()
            .context("run git worktree remove")?;

        if output.status.success() {
            return Ok(());
        }
        if snapshot_path.exists() {
            std::fs::remove_dir_all(snapshot_path).with_context(|| {
                format!(
                    "remove incomplete snapshot dir: {}",
                    snapshot_path.display()
                )
            })?;
        }
        let _ = self
            .git_command_blocking()
            .args(["worktree", "prune"])
            .output();
        Ok(())
    }
}

fn missing_commit_graph_object(message: &str) -> Option<String> {
    const PREFIX: &str = "You are attempting to fetch ";
    const SUFFIX: &str = ", which is in the commit graph file";

    message.lines().find_map(|line| {
        let start = line.find(PREFIX)? + PREFIX.len();
        let rest = &line[start..];
        let end = rest.find(SUFFIX)?;
        let object_id = &rest[..end];
        if matches!(object_id.len(), 40 | 64)
            && object_id.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            Some(object_id.to_string())
        } else {
            None
        }
    })
}

#[cfg(test)]
#[path = "snapshot_test.rs"]
mod tests;
