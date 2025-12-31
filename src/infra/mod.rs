//! Infrastructure layer (adapters/implementations).
//!
//! This module contains IO-heavy integrations (SQLite, ACP, filesystem).

pub mod acp;
pub mod app_config;
pub mod db;
pub mod diagram;
pub mod diff;
pub mod editor;
pub mod hash;
pub mod shell;
pub mod vcs;

/// Normalizes escaped or literal newlines to standard \n.
pub fn normalize_newlines(s: &str) -> String {
    s.replace("\\r\\n", "\n")
        .replace("\\n", "\n")
        .replace("\\r", "\n")
        .replace("\r\n", "\n")
}
