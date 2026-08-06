use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliStatus {
    pub is_installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
}

#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub async fn get_cli_status() -> Result<CliStatus, String> {
    let path = which::which("lareview").ok();
    let is_installed = path.is_some();
    let path_str = path.as_ref().map(|p| p.to_string_lossy().to_string());

    let version = if is_installed {
        let output = std::process::Command::new("lareview")
            .arg("--version")
            .output()
            .ok();

        output.and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                // "lareview 0.0.18" -> "0.0.18"
                Some(s.replace("lareview ", ""))
            } else {
                None
            }
        })
    } else {
        None
    };

    Ok(CliStatus {
        is_installed,
        version,
        path: path_str,
    })
}

#[tauri::command]
pub async fn install_cli() -> Result<(), String> {
    #[cfg(unix)]
    {
        let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;

        // Try /usr/local/bin first, fall back to ~/.local/bin on permission denied
        let primary_target = std::path::PathBuf::from("/usr/local/bin/lareview");
        let fallback_target = dirs::home_dir()
            .map(|h| h.join(".local/bin/lareview"))
            .ok_or_else(|| "Could not determine home directory".to_string())?;

        let install_to_target = |target: &std::path::Path| -> Result<(), std::io::Error> {
            // Ensure parent directory exists
            if let Some(parent) = target.parent()
                && !parent.exists()
            {
                std::fs::create_dir_all(parent)?;
            }

            // Check if it already exists and points to the same executable
            if target.exists() {
                if let Ok(existing) = std::fs::read_link(target)
                    && existing == current_exe
                {
                    return Ok(());
                }
                // Remove existing if it's different or not a symlink
                let _ = std::fs::remove_file(target);
            }

            std::os::unix::fs::symlink(&current_exe, target)
        };

        // Try primary location first
        match install_to_target(&primary_target) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                // Fall back to ~/.local/bin
                install_to_target(&fallback_target).map_err(|e| {
                    format!(
                        "Failed to install CLI. Could not write to /usr/local/bin or ~/.local/bin: {}",
                        e
                    )
                })
            }
            Err(e) => Err(format!("Failed to create symlink: {}", e)),
        }
    }

    #[cfg(not(unix))]
    Err("CLI installation is only supported on macOS and Linux".to_string())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: String,
    pub auto_refresh: bool,
    pub refresh_interval: u32,
    pub syntax_highlighting: bool,
    pub inline_comments: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackFilterConfig {
    pub confidence_threshold: Option<f64>,
}

#[tauri::command]
pub fn get_feedback_filter_config() -> FeedbackFilterConfig {
    use crate::infra::app_config::load_config;
    let config = load_config();
    FeedbackFilterConfig {
        confidence_threshold: config.feedback_confidence_threshold,
    }
}

#[tauri::command]
pub fn update_feedback_filter_config(threshold: Option<f64>) -> Result<(), String> {
    use crate::infra::app_config::{load_config, save_config};
    let mut config = load_config();
    // Clamp to valid range
    config.feedback_confidence_threshold = threshold.map(|t| t.clamp(0.0, 1.0));
    save_config(&config).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    pub timeout_secs: Option<u64>,
}

#[tauri::command]
pub fn get_timeout_config() -> TimeoutConfig {
    use crate::infra::app_config::load_config;
    let config = load_config();
    TimeoutConfig {
        timeout_secs: config.review_timeout_secs,
    }
}

#[tauri::command]
pub fn update_timeout_config(timeout_secs: Option<u64>) -> Result<(), String> {
    use crate::infra::app_config::{load_config, save_config};
    let mut config = load_config();
    config.review_timeout_secs = timeout_secs.map(|t| t.clamp(60, 7200));
    save_config(&config).map_err(|e| e.to_string())
}
