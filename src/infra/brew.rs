use std::path::PathBuf;

pub fn find_brew() -> Option<PathBuf> {
    let candidates = ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"];

    for path in candidates {
        let brew_path = PathBuf::from(path);
        if brew_path.is_file() {
            return Some(brew_path);
        }
    }

    which::which("brew").ok()
}
