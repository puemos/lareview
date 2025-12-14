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
}

impl ServerConfig {
    /// Parse server configuration from command-line arguments.
    pub fn from_args() -> Self {
        let mut config = ServerConfig::default();

        if let Ok(path) = std::env::var("TASK_MCP_OUT") {
            config.tasks_out = Some(PathBuf::from(path));
        }
        if let Ok(path) = std::env::var("TASK_MCP_RUN_CONTEXT") {
            config.run_context = Some(PathBuf::from(path));
        }

        let args: Vec<String> = std::env::args().collect();
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
                _ => i += 1,
            }
        }

        config
    }
}
