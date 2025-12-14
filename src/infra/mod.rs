//! Infrastructure layer (adapters/implementations).
//!
//! This module contains IO-heavy integrations (SQLite, ACP, filesystem).

pub mod acp;
pub mod db;
pub mod diff;
pub mod diff_index;
pub mod github;
pub mod hash;
