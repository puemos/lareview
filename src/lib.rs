pub mod application;
pub mod commands;
pub mod domain;
pub mod infra;
pub mod prompts;
pub mod state;

use std::future::Future;
use tokio::runtime::Runtime;

lazy_static::lazy_static! {
    static ref RUNTIME: Runtime = Runtime::new().expect("Failed to create Tokio runtime");
}

pub fn block_on<F: Future>(future: F) -> F::Output {
    RUNTIME.block_on(future)
}
