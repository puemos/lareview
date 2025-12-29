//! Transient UI state stored in egui's memory system.
//!
//! This module provides ephemeral UI state that persists between frames but
//! is not persisted to disk. Use this for drag handles, text drafts, form toggles,
//! and other purely visual state that doesn't need to survive app restarts.

use std::collections::HashMap;
use std::sync::Arc;

use eframe::egui;

use crate::infra::app_config::CustomAgentConfig;

/// egui Id for accessing UiMemory
pub const UI_MEMORY_ID: &str = "la_review_ui_memory";

/// UI state stored in egui's memory system.
/// Persists between frames, not persisted to disk.
#[derive(Default, Clone)]
pub struct UiMemory {
    /// Export overlay state
    pub export: ExportUiMemory,
    /// Feedback form drafts, keyed by feedback_id (or "new" for new feedback)
    pub feedback_drafts: HashMap<String, FeedbackDraftMemory>,
    /// Cached unified diff text, keyed by cache key (derived from task_id + diff_refs hash)
    pub cached_diffs: HashMap<String, Arc<str>>,
    /// Settings modal state
    pub settings: SettingsUiMemory,
}

/// Export overlay transient state
#[derive(Clone)]
pub struct ExportUiMemory {
    /// Sidebar width in pixels
    pub sidebar_width: f32,
}

impl Default for ExportUiMemory {
    fn default() -> Self {
        Self {
            sidebar_width: 300.0,
        }
    }
}

/// Feedback form draft state
#[derive(Default, Clone)]
pub struct FeedbackDraftMemory {
    /// Title draft text
    pub title: String,
    /// Reply draft text
    pub reply: String,
}

/// Settings modal transient state
#[derive(Default, Clone)]
pub struct SettingsUiMemory {
    /// Environment variable draft key
    pub agent_env_draft_key: String,
    /// Environment variable draft value
    pub agent_env_draft_value: String,
    /// Custom agent draft form
    pub custom_agent_draft: CustomAgentConfig,
    /// Agent ID being edited in settings
    pub editing_agent_id: Option<String>,
    /// Add custom agent modal visibility
    pub show_add_custom_agent_modal: bool,
    /// Snapshot of agent settings for dirty checking
    pub agent_settings_snapshot: Option<crate::ui::app::state::AgentSettingsSnapshot>,
}

impl UiMemory {
    /// Get feedback draft for a specific feedback ID, or "new" for new feedback
    pub fn get_feedback_draft(&self, feedback_id: &str) -> FeedbackDraftMemory {
        self.feedback_drafts
            .get(feedback_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Clear feedback draft after successful submission
    pub fn clear_feedback_draft(&mut self, feedback_id: &str) {
        self.feedback_drafts.remove(feedback_id);
    }

    /// Get cached diff for given cache key (typically task_id)
    pub fn get_cached_diff(&self, cache_key: &str) -> Option<Arc<str>> {
        self.cached_diffs.get(cache_key).cloned()
    }

    /// Cache a unified diff
    pub fn cache_diff(&mut self, cache_key: String, diff: Arc<str>) {
        self.cached_diffs.insert(cache_key, diff);
    }

    /// Generate a draft key for a feedback item
    pub fn feedback_draft_key(
        feedback_id: Option<&str>,
        task_id: &str,
        file_path: Option<&str>,
        line_number: Option<u32>,
    ) -> String {
        if let Some(id) = feedback_id {
            id.to_string()
        } else {
            format!("new:{}:{:?}:{:?}", task_id, file_path, line_number)
        }
    }
}

/// Get a reference to UiMemory from egui context
pub fn get_ui_memory(ctx: &egui::Context) -> UiMemory {
    ctx.memory(|mem| {
        mem.data
            .get_temp::<UiMemory>(egui::Id::new(UI_MEMORY_ID))
            .unwrap_or_default()
    })
}

/// Mutate UiMemory in egui context
pub fn with_ui_memory_mut<F, R>(ctx: &egui::Context, f: F) -> R
where
    F: FnOnce(&mut UiMemory) -> R,
{
    ctx.memory_mut(|mem| {
        let ui_mem = mem.data.get_temp_mut_or_insert_with::<UiMemory>(
            egui::Id::new(UI_MEMORY_ID),
            UiMemory::default,
        );
        f(ui_mem)
    })
}
