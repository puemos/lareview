use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorCandidate {
    pub id: String,
    pub label: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    pub preferred_editor_id: Option<String>,
}

#[tauri::command]
pub async fn get_available_editors() -> Vec<EditorCandidate> {
    crate::infra::editor::list_available_editors()
        .into_iter()
        .map(|e| EditorCandidate {
            id: e.id.to_string(),
            label: e.label.to_string(),
            path: e.path.to_string_lossy().to_string(),
        })
        .collect()
}

#[tauri::command]
pub fn get_editor_config() -> EditorConfig {
    use crate::infra::app_config::load_config;
    let config = load_config();
    EditorConfig {
        preferred_editor_id: config.preferred_editor_id,
    }
}

#[tauri::command]
pub fn update_editor_config(editor_id: String) -> Result<(), String> {
    use crate::infra::app_config::{load_config, save_config};
    let mut config = load_config();
    config.preferred_editor_id = Some(editor_id);
    save_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_in_editor(
    file_path: String,
    line_number: usize,
    repo_root: Option<String>,
) -> Result<(), String> {
    use crate::infra::app_config::load_config;
    use crate::infra::editor::{editor_command_for_open, is_editor_available};
    use std::path::PathBuf;
    use std::process::Command;

    let config = load_config();
    let editor_id = config
        .preferred_editor_id
        .ok_or_else(|| "No editor selected in settings".to_string())?;

    if !is_editor_available(&editor_id) {
        return Err(format!("Editor '{}' is not available", editor_id));
    }

    // Resolve absolute path if repo_root is provided
    let resolved_path = if let Some(root) = repo_root {
        PathBuf::from(root).join(&file_path)
    } else {
        PathBuf::from(&file_path)
    };

    if let Some((cmd_path, args)) = editor_command_for_open(&editor_id, &resolved_path, line_number)
    {
        Command::new(cmd_path)
            .args(args)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err(format!(
            "Could not construct open command for editor '{}'",
            editor_id
        ))
    }
}
