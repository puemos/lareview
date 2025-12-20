use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::OnceLock;

pub fn find_bin(command: &str) -> Option<PathBuf> {
    let path = std::path::Path::new(command);
    if path.components().count() > 1 && path.is_file() {
        return Some(path.to_path_buf());
    }

    let candidate_names = candidate_names(command);
    for dir in collect_search_paths() {
        for name in &candidate_names {
            let candidate = dir.join(name);
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

fn candidate_names(command: &str) -> Vec<OsString> {
    let mut names = Vec::new();
    names.push(OsString::from(command));

    #[cfg(target_os = "windows")]
    {
        if std::path::Path::new(command).extension().is_none() {
            let exts = std::env::var("PATHEXT")
                .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
            for ext in exts.split(';') {
                let ext = ext.trim();
                if ext.is_empty() {
                    continue;
                }
                names.push(OsString::from(format!("{command}{ext}")));
            }
        }
    }

    names
}

fn collect_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(extra) = std::env::var_os("LAREVIEW_EXTRA_PATH") {
        push_unique_paths(&mut paths, std::env::split_paths(&extra));
    }

    if let Some(env_path) = std::env::var_os("PATH") {
        push_unique_paths(&mut paths, std::env::split_paths(&env_path));
    }

    push_unique_paths(&mut paths, well_known_paths());

    #[cfg(target_os = "macos")]
    {
        push_unique_paths(&mut paths, path_helper_paths());
        push_unique_paths(&mut paths, launchctl_paths());
    }

    paths
}

fn push_unique_paths<I>(dest: &mut Vec<PathBuf>, paths: I)
where
    I: IntoIterator<Item = PathBuf>,
{
    for path in paths {
        if !dest.iter().any(|existing| existing == &path) {
            dest.push(path);
        }
    }
}

fn well_known_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(home) = home::home_dir() {
        let candidates = [
            home.join(".local/bin"),
            home.join(".cargo/bin"),
            home.join(".npm-global/bin"),
            home.join(".pnpm"),
            home.join(".yarn/bin"),
            home.join(".asdf/shims"),
            home.join(".pyenv/shims"),
            home.join(".rbenv/shims"),
            home.join(".nodenv/shims"),
            home.join(".poetry/bin"),
            home.join(".bun/bin"),
            home.join(".deno/bin"),
            home.join("bin"),
        ];
        paths.extend(candidates);
        paths.extend(mise_paths(&home));
    }

    #[cfg(target_os = "macos")]
    {
        let candidates = [
            "/opt/homebrew/bin",
            "/opt/homebrew/sbin",
            "/usr/local/bin",
            "/usr/local/sbin",
            "/usr/bin",
            "/bin",
            "/usr/sbin",
            "/sbin",
        ];
        paths.extend(candidates.iter().map(|p| PathBuf::from(p)));
    }

    #[cfg(target_os = "linux")]
    {
        let candidates = [
            "/usr/local/bin",
            "/usr/bin",
            "/bin",
            "/usr/sbin",
            "/sbin",
            "/snap/bin",
            "/var/lib/flatpak/exports/bin",
            "/var/lib/snapd/snap/bin",
        ];
        paths.extend(candidates.iter().map(|p| PathBuf::from(p)));
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let local = PathBuf::from(local);
            paths.push(local.join("Microsoft/WindowsApps"));
            paths.push(local.join("Programs/Git/bin"));
            paths.push(local.join("Programs/Python/Scripts"));
        }
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let appdata = PathBuf::from(appdata);
            paths.push(appdata.join("npm"));
        }
        if let Some(profile) = std::env::var_os("USERPROFILE") {
            let profile = PathBuf::from(profile);
            paths.push(profile.join(".cargo/bin"));
            paths.push(profile.join("bin"));
        }
    }

    paths
}

fn mise_paths(home: &std::path::Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let installs_dir = home.join(".local/share/mise/installs");
    let Ok(tool_dirs) = std::fs::read_dir(&installs_dir) else {
        return paths;
    };

    for tool_dir in tool_dirs.flatten() {
        let Ok(versions) = std::fs::read_dir(tool_dir.path()) else {
            continue;
        };
        for version_dir in versions.flatten() {
            let bin_dir = version_dir.path().join("bin");
            if bin_dir.is_dir() {
                paths.push(bin_dir);
            }
        }
    }

    paths
}

#[cfg(target_os = "macos")]
fn path_helper_paths() -> Vec<PathBuf> {
    static PATHS: OnceLock<Vec<PathBuf>> = OnceLock::new();
    PATHS
        .get_or_init(|| {
            let output = std::process::Command::new("/usr/libexec/path_helper")
                .arg("-s")
                .output();
            let Ok(output) = output else {
                return Vec::new();
            };
            if !output.status.success() {
                return Vec::new();
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Some(rest) = line.strip_prefix("PATH=\"") {
                    if let Some(end) = rest.find('\"') {
                        let value = &rest[..end];
                        return std::env::split_paths(value).collect();
                    }
                }
            }
            Vec::new()
        })
        .clone()
}

#[cfg(target_os = "macos")]
fn launchctl_paths() -> Vec<PathBuf> {
    static PATHS: OnceLock<Vec<PathBuf>> = OnceLock::new();
    PATHS
        .get_or_init(|| {
            let output = std::process::Command::new("/bin/launchctl")
                .args(["getenv", "PATH"])
                .output();
            let Ok(output) = output else {
                return Vec::new();
            };
            if !output.status.success() {
                return Vec::new();
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            let value = stdout.trim();
            if value.is_empty() {
                return Vec::new();
            }
            std::env::split_paths(value).collect()
        })
        .clone()
}
