//! Tauri IPC bridge.
//!
//! Commands are grouped by concern in the submodules below; this module only
//! declares and re-exports them.

pub mod agents;
pub mod diff;
pub mod editor;
pub mod feedback;
pub mod generation;
pub mod learning;
pub mod pending;
pub mod repos;
pub mod review;
pub mod rules;
pub mod settings;
pub mod system;
pub mod vcs;

pub use agents::*;
pub use diff::*;
pub use editor::*;
pub use feedback::*;
pub use generation::*;
pub use learning::*;
pub use pending::*;
pub use repos::*;
pub use review::*;
pub use rules::*;
pub use settings::*;
pub use system::*;
pub use vcs::*;
