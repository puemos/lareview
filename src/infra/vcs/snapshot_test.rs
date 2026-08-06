use super::{SnapshotManager, missing_commit_graph_object};
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use tempfile::TempDir;
use uuid::Uuid;

fn unique_session_id(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::new_v4())
}

fn init_git_repo(path: &Path) {
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .expect("failed to init git repo");

    // Configure user for commits
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(path)
        .output()
        .expect("failed to configure git email");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(path)
        .output()
        .expect("failed to configure git name");

    // Create a dummy file and commit it
    fs::write(path.join("file.txt"), "content").expect("failed to write file");
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .expect("failed to add files");

    std::process::Command::new("git")
        .args(["commit", "-m", "initial commit"])
        .current_dir(path)
        .output()
        .expect("failed to commit");
}

#[tokio::test]
async fn test_create_snapshot_success() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let repo_path = temp_dir.path();

    init_git_repo(repo_path);

    // Get the commit hash
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .expect("failed to get HEAD");
    let commit_sha = String::from_utf8(output.stdout).unwrap().trim().to_string();

    let manager = SnapshotManager::new(repo_path.to_path_buf());
    let session_id = unique_session_id("test-session");

    let snapshot_path: PathBuf = manager
        .create(&session_id, &commit_sha)
        .await
        .expect("failed to create snapshot");

    assert!(snapshot_path.exists());
    assert!(snapshot_path.join(".git").is_file());
    assert!(snapshot_path.join("file.txt").exists());
    assert_eq!(
        fs::read_to_string(snapshot_path.join("file.txt")).unwrap(),
        "content"
    );
    let worktrees = std::process::Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .expect("failed to list worktrees");
    assert!(String::from_utf8_lossy(&worktrees.stdout).contains(&*snapshot_path.to_string_lossy()));

    // Cleanup
    let result: anyhow::Result<()> = manager.remove(&snapshot_path).await;
    result.expect("failed to remove snapshot");
    assert!(!snapshot_path.exists());
    let worktrees = std::process::Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .expect("failed to list worktrees after cleanup");
    assert!(
        !String::from_utf8_lossy(&worktrees.stdout).contains(&*snapshot_path.to_string_lossy())
    );
}

#[tokio::test]
async fn test_create_snapshot_with_invalid_sha() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let repo_path = temp_dir.path();
    init_git_repo(repo_path);

    let manager = SnapshotManager::new(repo_path.to_path_buf());
    let session_id = unique_session_id("test-session-invalid");
    let result: anyhow::Result<PathBuf> = manager.create(&session_id, "invalidsha").await;

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("invalid Git object ID")
    );
    assert!(
        !std::env::temp_dir()
            .join("lareview-snapshots")
            .join(session_id)
            .exists()
    );
}

#[tokio::test]
async fn test_create_snapshot_rejects_session_path_traversal() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    init_git_repo(temp_dir.path());
    let commit_sha = String::from_utf8(
        std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(temp_dir.path())
            .output()
            .expect("failed to get HEAD")
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();

    let manager = SnapshotManager::new(temp_dir.path());
    let error = manager
        .create("../outside", &commit_sha)
        .await
        .expect_err("path traversal should be rejected");

    assert!(error.to_string().contains("single path component"));
}

#[test]
fn test_extracts_missing_commit_graph_object() {
    let object_id = "0dd9fc9a8855a0f044a10ba34d41e7240a82f202";
    let message = format!(
        "fatal: You are attempting to fetch {object_id}, which is in the commit graph file but not in the object database.\n\
         This is probably due to repo corruption."
    );

    assert_eq!(
        missing_commit_graph_object(&message).as_deref(),
        Some(object_id)
    );
    assert_eq!(missing_commit_graph_object("fatal: unrelated error"), None);
}

#[cfg(unix)]
#[tokio::test]
async fn test_create_snapshot_from_blobless_partial_clone() {
    let source_dir = TempDir::new().expect("failed to create source dir");
    init_git_repo(source_dir.path());
    for index in 0..32 {
        fs::write(
            source_dir.path().join(format!("file-{index}.txt")),
            format!("unique partial clone content {index}"),
        )
        .expect("failed to write source file");
    }
    let output = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(source_dir.path())
        .output()
        .expect("failed to add source files");
    assert!(output.status.success());
    let output = std::process::Command::new("git")
        .args(["commit", "-m", "add partial-clone fixtures"])
        .current_dir(source_dir.path())
        .output()
        .expect("failed to commit source files");
    assert!(output.status.success());
    let output = std::process::Command::new("git")
        .args(["config", "uploadpack.allowFilter", "true"])
        .current_dir(source_dir.path())
        .output()
        .expect("failed to enable partial-clone filters");
    assert!(output.status.success());

    let clone_parent = TempDir::new().expect("failed to create clone parent");
    let clone_path = clone_parent.path().join("partial");
    let source_url = format!("file://{}", source_dir.path().display());
    let output = std::process::Command::new("git")
        .args([
            "-c",
            "protocol.file.allow=always",
            "clone",
            "--quiet",
            "--filter=blob:none",
            "--no-checkout",
            &source_url,
        ])
        .arg(&clone_path)
        .output()
        .expect("failed to create partial clone");
    assert!(output.status.success());

    let commit_sha = String::from_utf8(
        std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&clone_path)
            .output()
            .expect("failed to get partial-clone HEAD")
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();
    let promisor = std::process::Command::new("git")
        .args(["config", "--get", "remote.origin.promisor"])
        .current_dir(&clone_path)
        .output()
        .expect("failed to inspect promisor config");
    assert_eq!(String::from_utf8_lossy(&promisor.stdout).trim(), "true");

    let manager = SnapshotManager::new(&clone_path);
    let snapshot_path = manager
        .create(&unique_session_id("partial-clone"), &commit_sha)
        .await
        .expect("failed to create snapshot from partial clone");

    assert_eq!(
        fs::read_to_string(snapshot_path.join("file-31.txt")).unwrap(),
        "unique partial clone content 31"
    );
    manager
        .remove(&snapshot_path)
        .await
        .expect("failed to remove partial-clone snapshot");
}
