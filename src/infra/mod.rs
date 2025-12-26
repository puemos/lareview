//! Infrastructure layer (adapters/implementations).
//!
//! This module contains IO-heavy integrations (SQLite, ACP, filesystem).

pub mod acp;
pub mod app_config;
pub mod d2;
pub mod db;
pub mod diff;
pub mod diff_index;
pub mod git;
pub mod github;
pub mod hash;
pub mod shell;

/// Normalizes escaped or literal newlines to standard \n.
pub fn normalize_newlines(s: &str) -> String {
    s.replace("\\r\\n", "\n")
        .replace("\\n", "\n")
        .replace("\\r", "\n")
        .replace("\r\n", "\n")
}
