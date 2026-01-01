#![allow(unexpected_cfgs)]
pub mod application;
pub mod assets;
pub mod domain;
pub mod infra;
pub mod prompts;
pub mod ui;

use std::sync::OnceLock;

pub use tokio::runtime::Runtime;

/// Global Tokio runtime handle for async operations throughout the application.
///
/// This is a temporary solution. In a future refactor, the runtime should be:
/// 1. Created in main() and passed through dependency injection
/// 2. Or stored in the LaReviewApp struct for UI-based operations
///
/// # Panics
///
/// All methods will panic if the runtime has not been initialized first.
/// Call `init_runtime()` or `set_runtime()` before using.
pub static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Initializes the global Tokio runtime.
///
/// # Errors
///
/// Returns an error if the runtime has already been initialized.
pub fn set_runtime(rt: Runtime) -> Result<(), Runtime> {
    RUNTIME.set(rt)
}

/// Returns a reference to the global Tokio runtime.
///
/// # Panics
///
/// Panics if the runtime has not been initialized.
pub fn runtime() -> &'static Runtime {
    RUNTIME
        .get()
        .expect("Tokio runtime not initialized. Call init_runtime() first.")
}

/// Spawns an async task on the global runtime.
///
/// This is a convenience wrapper around `tokio::spawn` that uses the global runtime.
///
/// # Panics
///
/// Panics if the global runtime has not been initialized.
pub fn spawn<T>(task: T) -> tokio::task::JoinHandle<T::Output>
where
    T: std::future::Future + Send + 'static,
    T::Output: Send + 'static,
{
    runtime().spawn(task)
}

/// Spawns a blocking task on the global runtime's blocking thread pool.
///
/// # Panics
///
/// Panics if the global runtime has not been initialized.
pub fn spawn_blocking<T, F>(task: F) -> tokio::task::JoinHandle<T>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    runtime().spawn_blocking(task)
}

/// Runs a blocking operation on the current thread.
///
/// This is useful for operations that must run synchronously but need to be
/// callable from async code.
///
/// # Panics
///
/// Panics if the global runtime has not been initialized.
pub fn block_on<F: std::future::Future>(future: F) -> F::Output {
    runtime().block_on(future)
}

/// An RAII guard that enters the Tokio runtime context.
///
/// When dropped, exits the runtime context. This is used in main() to ensure
/// the runtime is active for the duration of the program.
pub fn enter_runtime() -> tokio::runtime::EnterGuard<'static> {
    runtime().enter()
}

/// Initializes the process path (shell environment) in a blocking manner.
///
/// This should be called early in main() before any async operations.
pub fn init_process_path() {
    let rt = Runtime::new().expect("Failed to create blocking runtime for init");
    rt.block_on(async {
        crate::infra::shell::init_process_path();
    })
}
