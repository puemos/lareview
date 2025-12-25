use std::path::PathBuf;

/// Configuration for the MCP server, parsed from CLI arguments.
#[derive(Debug, Clone, Default)]
pub struct ServerConfig {
    /// Optional path to write tasks JSON output.
    pub tasks_out: Option<PathBuf>,
    /// Optional path for debug log file.
    pub log_file: Option<PathBuf>,
    /// Optional path to review/run context JSON file.
    pub run_context: Option<PathBuf>,
    /// Optional repo root for read-only tooling (search).
    pub repo_root: Option<PathBuf>,
    /// Optional database path override.
    pub db_path: Option<PathBuf>,
}

impl ServerConfig {
    /// Parse server configuration from command-line arguments.
    pub fn from_args() -> Self {
        let args: Vec<String> = std::env::args().collect();
        Self::from_iter(args)
    }

    /// Parse server configuration from an iterator of strings.
    pub fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let mut config = ServerConfig::default();

        if let Ok(path) = std::env::var("TASK_MCP_OUT") {
            config.tasks_out = Some(PathBuf::from(path));
        }
        if let Ok(path) = std::env::var("TASK_MCP_RUN_CONTEXT") {
            config.run_context = Some(PathBuf::from(path));
        }
        if let Ok(path) = std::env::var("TASK_MCP_REPO_ROOT") {
            config.repo_root = Some(PathBuf::from(path));
        }
        if let Ok(path) =
            std::env::var("TASK_MCP_DB_PATH").or_else(|_| std::env::var("LAREVIEW_DB_PATH"))
        {
            config.db_path = Some(PathBuf::from(path));
        }

        let args: Vec<String> = iter.into_iter().collect();
        let mut i = 0;

        while i < args.len() {
            match args[i].as_str() {
                "--tasks-out" => {
                    if i + 1 < args.len() {
                        config.tasks_out = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--log-file" => {
                    if i + 1 < args.len() {
                        config.log_file = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--pr-context" => {
                    if i + 1 < args.len() {
                        config.run_context = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--repo-root" => {
                    if i + 1 < args.len() {
                        config.repo_root = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--db-path" => {
                    if i + 1 < args.len() {
                        config.db_path = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                _ => i += 1,
            }
        }

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_from_iter() {
        let args = vec![
            "program_name".to_string(),
            "--tasks-out".to_string(),
            "/tmp/tasks.json".to_string(),
            "--log-file".to_string(),
            "/tmp/debug.log".to_string(),
            "--pr-context".to_string(),
            "/tmp/context.json".to_string(),
            "--repo-root".to_string(),
            "/tmp/repo".to_string(),
            "--db-path".to_string(),
            "/tmp/test.db".to_string(),
        ];

        let config = ServerConfig::from_iter(args);

        assert_eq!(config.tasks_out, Some(PathBuf::from("/tmp/tasks.json")));
        assert_eq!(config.log_file, Some(PathBuf::from("/tmp/debug.log")));
        assert_eq!(config.run_context, Some(PathBuf::from("/tmp/context.json")));
        assert_eq!(config.repo_root, Some(PathBuf::from("/tmp/repo")));
        assert_eq!(config.db_path, Some(PathBuf::from("/tmp/test.db")));
    }

    #[test]
    fn test_server_config_incomplete_args() {
        let args = vec!["program_name".to_string(), "--tasks-out".to_string()];
        let config = ServerConfig::from_iter(args);
        assert_eq!(config.tasks_out, None);
    }
}
