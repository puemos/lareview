use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CustomAgentConfig {
    pub id: String,
    pub label: String,
    pub logo: Option<String>,
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub has_seen_requirements: bool,
    pub custom_agents: Vec<CustomAgentConfig>,
    pub agent_path_overrides: HashMap<String, String>,
    pub agent_args_overrides: HashMap<String, Vec<String>>,
    pub agent_envs: HashMap<String, HashMap<String, String>>,
    pub preferred_editor_id: Option<String>,
    /// Minimum confidence threshold for displaying feedback (0.0-1.0)
    /// None means show all feedback (default behavior)
    #[serde(default)]
    pub feedback_confidence_threshold: Option<f64>,
    /// Review generation timeout in seconds.
    /// None means use the built-in default of 1000 seconds.
    #[serde(default)]
    pub review_timeout_secs: Option<u64>,
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return AppConfig::default();
    };
    toml::from_str(&contents).unwrap_or_default()
}

pub fn save_config(config: &AppConfig) -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents = toml::to_string_pretty(config).unwrap_or_default();
    std::fs::write(path, contents)
}

fn config_path() -> PathBuf {
    if let Ok(path) = std::env::var("LAREVIEW_CONFIG_PATH") {
        return PathBuf::from(path);
    }

    app_data_dir().join("config.toml")
}

fn app_data_dir() -> PathBuf {
    if let Ok(path) = std::env::var("LAREVIEW_DATA_HOME") {
        return PathBuf::from(path);
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = home::home_dir() {
            return home
                .join("Library")
                .join("Application Support")
                .join("LaReview");
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata).join("LaReview");
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
            return PathBuf::from(xdg).join("lareview");
        }
        if let Some(home) = home::home_dir() {
            return home.join(".local").join("share").join("lareview");
        }
    }

    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".lareview")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::NamedTempFile;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_config_serialization() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let mut agent_envs = HashMap::new();
        let mut codex_envs = HashMap::new();
        codex_envs.insert("API_KEY".to_string(), "secret".to_string());
        agent_envs.insert("codex".to_string(), codex_envs);

        let mut path_overrides = HashMap::new();
        path_overrides.insert("gemini".to_string(), "/custom/gemini".to_string());

        let mut args_overrides = HashMap::new();
        args_overrides.insert(
            "gemini".to_string(),
            vec!["--verbose".to_string(), "--fast".to_string()],
        );

        let config = AppConfig {
            has_seen_requirements: true,
            custom_agents: vec![CustomAgentConfig {
                id: "my-agent".into(),
                label: "My Agent".into(),
                logo: None,
                command: "my-command".into(),
                args: vec!["--flag".into()],
                env_vars: {
                    let mut m = HashMap::new();
                    m.insert("MY_VAR".to_string(), "my_value".to_string());
                    m
                },
            }],
            agent_path_overrides: path_overrides,
            agent_args_overrides: args_overrides,
            agent_envs,
            preferred_editor_id: Some("vscode".into()),
            feedback_confidence_threshold: None,
            review_timeout_secs: None,
        };

        let tmp_file = NamedTempFile::new().unwrap();
        let path = tmp_file.path().to_path_buf();

        // Manual save using our path
        let contents = toml::to_string_pretty(&config).unwrap();
        std::fs::write(&path, contents).unwrap();

        // Test loading - unsafe but protected by ENV_MUTEX in test context
        unsafe {
            std::env::set_var("LAREVIEW_CONFIG_PATH", &path);
        }
        let loaded = load_config();
        assert!(loaded.has_seen_requirements);
        assert_eq!(loaded.custom_agents.len(), 1);
        assert_eq!(loaded.custom_agents[0].id, "my-agent");
        assert_eq!(
            loaded.agent_path_overrides.get("gemini").unwrap(),
            "/custom/gemini"
        );
        let loaded_args = loaded.agent_args_overrides.get("gemini").unwrap();
        assert_eq!(loaded_args.len(), 2);
        assert_eq!(loaded_args[0], "--verbose");

        assert_eq!(
            loaded
                .agent_envs
                .get("codex")
                .unwrap()
                .get("API_KEY")
                .unwrap(),
            "secret"
        );
        assert_eq!(loaded.preferred_editor_id.as_deref(), Some("vscode"));

        // Test saving
        save_config(&loaded).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("my-agent"));
        assert!(contents.contains("API_KEY"));

        // Cleanup - unsafe but protected by ENV_MUTEX in test context
        unsafe {
            std::env::remove_var("LAREVIEW_CONFIG_PATH");
        }
    }

    #[test]
    fn test_load_config_missing() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let path = PathBuf::from("/non/existent/path/to/config.toml");
        // Test loading missing config - unsafe but protected by ENV_MUTEX
        unsafe {
            std::env::set_var("LAREVIEW_CONFIG_PATH", &path);
        }
        let config = load_config();
        assert!(!config.has_seen_requirements);
        unsafe {
            std::env::remove_var("LAREVIEW_CONFIG_PATH");
        }
    }
}
