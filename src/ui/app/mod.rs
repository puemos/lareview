//! Main application state and UI logic for the LaReview application.
//!
//! This module contains the primary egui application state, async message plumbing, and the root
//! `eframe::App` implementation.

mod generate_input;
mod header;
mod init;
mod messages;
mod overlay;
mod polling;
mod review_ops;
mod root;
pub(crate) mod state;
mod store;
mod timeline;
pub mod ui_memory;
mod update;

#[cfg(test)]
pub mod tests;

pub use messages::{GenMsg, GenResultPayload, GhMsg, GhStatusPayload};
pub use root::LaReviewApp;
pub use state::{
    AgentSettingsSnapshot, AppView, DomainState, FeedbackContext, FullDiffView, GeneratePreview,
    SelectedAgent, UiState,
};
pub use timeline::{TimelineContent, TimelineItem};

pub(crate) use store::{
    Action, AsyncAction, GenerateAction, NavigationAction, ReviewAction, SettingsAction,
};
