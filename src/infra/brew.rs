use std::path::PathBuf;

pub fn find_bin(command: &str) -> Option<PathBuf> {
    let path = std::path::Path::new(command);
    if path.components().count() > 1 && path.is_file() {
        return Some(path.to_path_buf());
    }

    if let Ok(path) = which::which(command) {
        return Some(path);
    }

    #[cfg(target_os = "macos")]
    {
        let candidates = ["/opt/homebrew/bin", "/usr/local/bin"];
        for base in candidates {
            let candidate = PathBuf::from(base).join(command);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

pub fn find_brew() -> Option<PathBuf> {
    find_bin("brew")
}
