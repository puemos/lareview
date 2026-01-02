//! Unified diff display component for LaReview.
//!
//! Handles parsing, rendering, and interaction with git diffs in a unified format with syntax
//! highlighting, inline diffs, and collapsible file sections.

mod doc;
mod indexer;
mod model;
pub mod overlay;
mod render;
pub mod syntax;

pub use model::{DiffAction, LineContext};
pub use render::{
    render_diff_editor, render_diff_editor_full_view, render_diff_editor_with_comment_callback,
    render_diff_editor_with_options,
};
