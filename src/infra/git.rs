use std::path::Path;
use std::process::Command;

/// Extract remote URLs from a git repository
pub fn extract_git_remotes(repo_path: &Path) -> Vec<String> {
    let output = Command::new("git")
        .args(["-C", &repo_path.to_string_lossy(), "remote", "-v"])
        .output()
        .ok();

    let mut remotes = Vec::new();
    if let Some(output) = output
        && output.status.success()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let url = parts[1].to_string();
                if !remotes.contains(&url) {
                    remotes.push(url);
                }
            }
        }
    }
    remotes
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_extract_git_remotes_empty() {
        let dir = tempdir().unwrap();
        // Not a git repo, or at least no remotes
        let remotes = extract_git_remotes(dir.path());
        assert!(remotes.is_empty());
    }

    #[test]
    fn test_extract_git_remotes_mock() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path();

        // Initialize a git repo and add a remote
        let status = std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .status()
            .unwrap();
        if !status.success() {
            return; // Skip if git is not installed or failed
        }

        std::process::Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "https://github.com/example/repo.git",
            ])
            .current_dir(repo_path)
            .status()
            .unwrap();

        let remotes = extract_git_remotes(repo_path);
        assert_eq!(remotes.len(), 1);
        assert_eq!(remotes[0], "https://github.com/example/repo.git");
    }
}
