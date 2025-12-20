use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub extra_path: Option<String>,
    pub has_seen_requirements: bool,
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
