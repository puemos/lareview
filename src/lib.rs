#![allow(unexpected_cfgs)]
pub mod application;
pub mod assets;
pub mod domain;
pub mod infra;
pub mod prompts;
pub mod ui;

use std::sync::OnceLock;

/// Global Tokio runtime handle for async operations throughout the application
pub static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
