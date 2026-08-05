use crate::infra::acp::invalidate_agent_cache;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[tauri::command]
pub async fn get_agents(_state: State<'_, AppState>) -> Result<Vec<AgentInfo>, String> {
    let candidates = crate::infra::acp::list_agent_candidates();
    let config = crate::infra::app_config::load_config();
    let custom_ids: std::collections::HashSet<&str> =
        config.custom_agents.iter().map(|c| c.id.as_str()).collect();

    let agents: Vec<AgentInfo> = candidates
        .into_iter()
        .map(|candidate| {
            let is_custom = custom_ids.contains(candidate.id.as_str());
            AgentInfo {
                id: candidate.id,
                name: candidate.label,
                description: None,
                path: candidate.command,
                args: candidate.args,
                logo: candidate.logo,
                available: candidate.available,
                is_custom,
            }
        })
        .collect();

    Ok(agents)
}

#[tauri::command]
pub fn update_agent_config(
    _state: State<'_, AppState>,
    id: String,
    path: String,
    args: Option<Vec<String>>,
) -> Result<(), String> {
    use crate::infra::app_config::{load_config, save_config};
    let mut config = load_config();

    // Check if it's a known built-in agent by checking its registry label/id
    // Actually, we can just check if it's in custom_agents first.
    let mut found_custom = false;
    for custom in config.custom_agents.iter_mut() {
        if custom.id == id {
            custom.command = path.clone();
            if let Some(new_args) = &args {
                custom.args = new_args.clone();
            }
            found_custom = true;
            break;
        }
    }

    if !found_custom {
        // Assume it's a built-in agent (or one we want to override)
        config.agent_path_overrides.insert(id.clone(), path);
        if let Some(new_args) = args {
            config.agent_args_overrides.insert(id, new_args);
        }
    }

    save_config(&config).map_err(|e| e.to_string())?;

    // Invalidate discovery cache so next get_agents or generation uses new path
    invalidate_agent_cache();

    Ok(())
}

#[tauri::command]
pub fn add_custom_agent(
    id: String,
    label: String,
    command: String,
    args: Option<Vec<String>>,
    logo: Option<String>,
) -> Result<(), String> {
    use crate::infra::app_config::{CustomAgentConfig, load_config, save_config};

    if id.trim().is_empty() {
        return Err("Agent ID cannot be empty".into());
    }
    if command.trim().is_empty() {
        return Err("Agent command cannot be empty".into());
    }

    let mut config = load_config();

    // Check for duplicate IDs across custom agents
    if config.custom_agents.iter().any(|c| c.id == id) {
        return Err(format!("A custom agent with ID '{}' already exists", id));
    }

    // Check for duplicate IDs across built-in agents
    let candidates = crate::infra::acp::list_agent_candidates();
    if candidates.iter().any(|c| c.id == id) {
        return Err(format!("An agent with ID '{}' already exists", id));
    }

    config.custom_agents.push(CustomAgentConfig {
        id,
        label,
        logo,
        command,
        args: args.unwrap_or_default(),
        env_vars: Default::default(),
    });

    save_config(&config).map_err(|e| e.to_string())?;
    invalidate_agent_cache();

    Ok(())
}

#[tauri::command]
pub fn delete_custom_agent(id: String) -> Result<(), String> {
    use crate::infra::app_config::{load_config, save_config};

    let mut config = load_config();

    let before_len = config.custom_agents.len();
    config.custom_agents.retain(|c| c.id != id);

    if config.custom_agents.len() == before_len {
        return Err(format!(
            "Cannot delete agent '{}': not a custom agent or does not exist",
            id
        ));
    }

    // Clean up any agent_envs entries for this custom agent
    config.agent_envs.remove(&id);

    save_config(&config).map_err(|e| e.to_string())?;
    invalidate_agent_cache();

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    pub args: Vec<String>,
    pub logo: Option<String>,
    pub available: bool,
    #[serde(default)]
    pub is_custom: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInput {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}
