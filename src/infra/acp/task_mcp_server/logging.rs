use super::config::ServerConfig;
use chrono::Local;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

fn timestamped_line(message: &str) -> String {
    let now = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    format!("[{now}] {message}")
}

fn default_log_path() -> PathBuf {
    let date = Local::now().format("%Y-%m-%d").to_string();
    let dir = PathBuf::from(".lareview/logs");
    let _ = fs::create_dir_all(&dir);
    dir.join(format!("mcp-{date}.log"))
}

fn resolve_log_path(config: &ServerConfig) -> PathBuf {
    if let Some(ref path) = config.log_file {
        return path.clone();
    }
    default_log_path()
}

pub(super) fn log_to_file(config: &ServerConfig, message: &str) {
    let path = resolve_log_path(config);
    let line = timestamped_line(message);
    let _ = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| writeln!(f, "{line}"));
}

/// Lightweight logging for protocol debugging; does not require config.
pub(super) fn log_raw_line(line: &str) {
    let path = default_log_path();
    let line = timestamped_line(line);
    let _ = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| writeln!(f, "{line}"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_log_to_file() {
        let tmp = NamedTempFile::new().unwrap();
        let config = ServerConfig {
            log_file: Some(tmp.path().to_path_buf()),
            ..Default::default()
        };

        log_to_file(&config, "test message");

        let contents = fs::read_to_string(tmp.path()).unwrap();
        assert!(contents.contains("test message"));
    }
}
