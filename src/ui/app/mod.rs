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
mod state;
mod store;
mod timeline;
mod update;

pub use messages::{GenMsg, GenResultPayload, GhMsg, GhStatusPayload};
pub use root::LaReviewApp;
pub use state::{FullDiffView, LineNoteContext, SelectedAgent, ThreadContext};
pub use timeline::{TimelineContent, TimelineItem};

pub(crate) use store::{
    Action, AsyncAction, GenerateAction, NavigationAction, ReviewAction, SettingsAction,
};
