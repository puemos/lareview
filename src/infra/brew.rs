use std::ffi::OsString;
use std::path::PathBuf;

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
    #[cfg(target_os = "windows")]
    {
        let mut names = vec![OsString::from(command)];
        if std::path::Path::new(command).extension().is_none() {
            let exts =
                std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
            for ext in exts.split(';') {
                let ext = ext.trim();
                if ext.is_empty() {
                    continue;
                }
                names.push(OsString::from(format!("{command}{ext}")));
            }
        }
        names
    }
    #[cfg(not(target_os = "windows"))]
    {
        vec![OsString::from(command)]
    }
}

fn collect_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(extra) = std::env::var_os("LAREVIEW_EXTRA_PATH") {
        push_unique_paths(&mut paths, std::env::split_paths(&extra));
    }

    if let Some(env_path) = std::env::var_os("PATH") {
        push_unique_paths(&mut paths, std::env::split_paths(&env_path));
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
