use super::config::ServerConfig;
use chrono::Local;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

fn default_log_path() -> PathBuf {
    let date = Local::now().format("%Y-%m-%d").to_string();
    let dir = PathBuf::from(".lareview/logs");
    let _ = fs::create_dir_all(&dir);
    dir.join(format!("mcp-{date}.log"))
}

pub(super) fn log_to_file(_config: &ServerConfig, message: &str) {
    let path = default_log_path();
    let _ = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| writeln!(f, "{message}"));
}
