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
