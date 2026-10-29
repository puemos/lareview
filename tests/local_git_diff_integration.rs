use lareview::domain::ReviewSource;
use lareview::infra::vcs::{git::compare_refs, snapshot::SnapshotManager};
use std::path::Path;
use std::process::Command;

fn git(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "tag.gpgsign=false",
        ])
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-b", "main"]);
    git(dir.path(), &["config", "user.name", "Local Git Test"]);
    git(
        dir.path(),
        &["config", "user.email", "test@example.invalid"],
    );
    std::fs::write(dir.path().join("hello.txt"), "hello\n").unwrap();
    git(dir.path(), &["add", "."]);
    git(dir.path(), &["commit", "-m", "base"]);
    git(dir.path(), &["tag", "v1"]);
    git(dir.path(), &["switch", "-c", "feature/on-prem"]);
    std::fs::write(dir.path().join("hello.txt"), "hello on-prem\n").unwrap();
    git(dir.path(), &["commit", "-am", "head"]);
    // An unreachable on-prem host proves comparison does not need a host API.
    git(
        dir.path(),
        &[
            "remote",
            "add",
            "origin",
            "ssh://git@code.corp.invalid:7999/team/repo.git",
        ],
    );
    dir
}

#[tokio::test]
async fn compares_local_commits_and_snapshots_the_selected_head_without_touching_working_changes() {
    let dir = fixture();
    std::fs::write(dir.path().join("hello.txt"), "dirty working tree\n").unwrap();
    git(dir.path(), &["add", "hello.txt"]);
    let diff = compare_refs(dir.path(), "v1", "feature/on-prem")
        .await
        .unwrap();
    assert!(diff.diff_text.contains("-hello\n+hello on-prem"));
    assert!(!diff.diff_text.contains("dirty working tree"));
    assert_eq!(diff.base_sha, git(dir.path(), &["rev-parse", "main"]));
    assert_eq!(diff.head_sha, git(dir.path(), &["rev-parse", "HEAD"]));

    let source = ReviewSource::LocalGit {
        repo_id: "linked-on-prem".into(),
        base_ref: "v1".into(),
        head_ref: "feature/on-prem".into(),
        base_sha: diff.base_sha,
        head_sha: diff.head_sha.clone(),
    };
    let serialized = serde_json::to_string(&source).unwrap();
    assert_eq!(
        serde_json::from_str::<ReviewSource>(&serialized).unwrap(),
        source
    );
    assert_eq!(source.head_sha(), Some(diff.head_sha.clone()));
    assert_eq!(source.provider_id(), None);
    assert_eq!(source.url(), None);

    let manager = SnapshotManager::new(dir.path());
    let id = uuid::Uuid::new_v4().to_string();
    let snapshot = manager.create(&id, &diff.head_sha).await.unwrap();
    assert_eq!(
        std::fs::read_to_string(snapshot.join("hello.txt")).unwrap(),
        "hello on-prem\n"
    );
    manager.remove(&snapshot).await.unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("hello.txt")).unwrap(),
        "dirty working tree\n"
    );
    assert!(!git(dir.path(), &["diff", "--cached"]).is_empty());
}

#[tokio::test]
async fn rejects_invalid_non_commit_and_option_refs_and_reports_identical_commits() {
    let dir = fixture();
    for base in [
        "",
        "--output=/tmp/should-not-exist",
        "missing-branch",
        "HEAD:hello.txt",
        "main..HEAD",
    ] {
        assert!(
            compare_refs(dir.path(), base, "HEAD").await.is_err(),
            "accepted {base}"
        );
    }
    let error = compare_refs(dir.path(), "HEAD", "HEAD")
        .await
        .err()
        .unwrap();
    assert!(error.to_string().contains("No changes between"));
    let non_repo = tempfile::tempdir().unwrap();
    assert!(compare_refs(non_repo.path(), "main", "HEAD").await.is_err());
}

#[tokio::test]
async fn disables_external_diff_helpers() {
    let dir = fixture();
    git(
        dir.path(),
        &[
            "config",
            "diff.external",
            "a-diff-helper-that-does-not-exist",
        ],
    );
    assert!(
        compare_refs(dir.path(), "main", "HEAD")
            .await
            .unwrap()
            .diff_text
            .contains("+hello on-prem")
    );
}
