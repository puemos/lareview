use crate::infra::vcs::snapshot::SnapshotManager;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use tempfile::TempDir;

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
    let session_id = "test-session";

    let snapshot_path: PathBuf = manager
        .create(session_id, &commit_sha)
        .await
        .expect("failed to create snapshot");

    assert!(snapshot_path.exists());
    assert!(snapshot_path.join("file.txt").exists());
    assert_eq!(
        fs::read_to_string(snapshot_path.join("file.txt")).unwrap(),
        "content"
    );

    // Cleanup
    let result: anyhow::Result<()> = manager.remove(&snapshot_path).await;
    result.expect("failed to remove snapshot");
    assert!(!snapshot_path.exists());
}

#[tokio::test]
async fn test_create_snapshot_with_invalid_sha() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let repo_path = temp_dir.path();
    init_git_repo(repo_path);

    let manager = SnapshotManager::new(repo_path.to_path_buf());
    let result: anyhow::Result<PathBuf> =
        manager.create("test-session-invalid", "invalidsha").await;

    // It should fail because read-tree will fail (fetch might fail silently or warn, but read-tree must find the object)
    assert!(result.is_err());
}
