use crate::infra::acp::list_agent_candidates;
use crate::state::AppState;
use tauri::State;

// ============================================================================
// Learning System Commands
// ============================================================================

use crate::domain::{
    LearnedPattern, LearnedPatternInput, LearningCompactionResult, LearningStatus,
};

/// Get all learned patterns
#[tauri::command]
pub fn get_learned_patterns(state: State<'_, AppState>) -> Result<Vec<LearnedPattern>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.learned_pattern_repo()
        .list_all()
        .map_err(|e| e.to_string())
}

/// Create a new learned pattern manually
#[tauri::command]
pub fn create_learned_pattern(
    state: State<'_, AppState>,
    input: LearnedPatternInput,
) -> Result<LearnedPattern, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.learned_pattern_repo()
        .create(&input, 0) // source_count = 0 for manual creation
        .map_err(|e| e.to_string())
}

/// Update an existing learned pattern
#[tauri::command]
pub fn update_learned_pattern(
    state: State<'_, AppState>,
    id: String,
    input: LearnedPatternInput,
) -> Result<LearnedPattern, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.learned_pattern_repo()
        .update(&id, &input)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Pattern not found".to_string())
}

/// Delete a learned pattern
#[tauri::command]
pub fn delete_learned_pattern(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let rows = db
        .learned_pattern_repo()
        .delete(&id)
        .map_err(|e| e.to_string())?;
    if rows == 0 {
        return Err("Pattern not found".to_string());
    }
    Ok(())
}

/// Toggle a learned pattern's enabled status
#[tauri::command]
pub fn toggle_learned_pattern(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let rows = db
        .learned_pattern_repo()
        .toggle_enabled(&id, enabled)
        .map_err(|e| e.to_string())?;
    if rows == 0 {
        return Err("Pattern not found".to_string());
    }
    Ok(())
}

/// Get the learning system status
#[tauri::command]
pub fn get_learning_status(state: State<'_, AppState>) -> Result<LearningStatus, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let pattern_repo = db.learned_pattern_repo();
    let state_repo = db.learning_state_repo();
    state_repo
        .get_status(&pattern_repo)
        .map_err(|e| e.to_string())
}

/// Trigger learning compaction manually
#[tauri::command]
pub async fn trigger_learning_compaction(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<LearningCompactionResult, String> {
    // Get the agent configuration
    let (agent_command, agent_args) = {
        let candidates = list_agent_candidates();
        let agent_candidate = candidates
            .iter()
            .find(|c| c.id == agent_id)
            .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;

        let command = agent_candidate.command.clone().ok_or_else(|| {
            format!(
                "Agent '{}' is not available. Please configure it in settings.",
                agent_id
            )
        })?;

        (command, agent_candidate.args.clone())
    };

    // Get unprocessed rejections and existing patterns
    let (rejections, existing_patterns, db_clone) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let rejection_repo = db.rejection_repo();
        let pattern_repo = db.learned_pattern_repo();

        let rejections = rejection_repo
            .get_unprocessed_rejections(50)
            .map_err(|e| e.to_string())?;

        let existing_patterns = pattern_repo.list_enabled().map_err(|e| e.to_string())?;

        // We need a clone of the Arc for the async operation
        drop(db);
        let db_clone = state.db.clone();

        (rejections, existing_patterns, db_clone)
    };

    if rejections.is_empty() {
        return Ok(LearningCompactionResult {
            rejections_processed: 0,
            patterns_created: 0,
            patterns_updated: 0,
            errors: vec!["No unprocessed rejections to analyze".to_string()],
        });
    }

    let input = crate::infra::acp::LearningCompactionInput {
        rejections,
        existing_patterns,
        agent_command,
        agent_args,
        db: db_clone,
        timeout_secs: Some(300),
        mcp_server_binary: None,
        cancel_token: None,
        debug: false,
    };

    crate::infra::acp::run_learning_compaction(input)
        .await
        .map_err(|e| e.to_string())
}
