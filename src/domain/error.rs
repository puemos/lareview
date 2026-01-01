//! Domain error types for LaReview application.
//!
//! These errors represent domain-level failures that can occur during
//! business operations. They are more specific than infrastructure errors
//! and can be handled appropriately at the application layer.

use thiserror::Error;

/// Domain errors related to review operations.
#[derive(Debug, Error)]
pub enum ReviewError {
    #[error("Review not found: {0}")]
    NotFound(String),

    #[error("Invalid review state: {0}")]
    InvalidState(String),

    #[error("Review operation failed: {0}")]
    OperationFailed(#[from] anyhow::Error),
}

/// Domain errors related to task operations.
#[derive(Debug, Error)]
pub enum TaskError {
    #[error("Task not found: {0}")]
    NotFound(String),

    #[error("Invalid task status transition from {current} to {next}")]
    InvalidStatusTransition { current: String, next: String },

    #[error("Task operation failed: {0}")]
    OperationFailed(#[from] anyhow::Error),
}

/// Domain errors related to feedback operations.
#[derive(Debug, Error)]
pub enum FeedbackError {
    #[error("Feedback not found: {0}")]
    NotFound(String),

    #[error("Invalid feedback anchor: file={file}, line={line}")]
    InvalidAnchor { file: String, line: u32 },

    #[error("Feedback operation failed: {0}")]
    OperationFailed(#[from] anyhow::Error),
}

/// Domain errors related to repository operations.
#[derive(Debug, Error)]
pub enum RepoError {
    #[error("Repository not found: {0}")]
    NotFound(String),

    #[error("Invalid repository path: {0}")]
    InvalidPath(String),

    #[error("Repository operation failed: {0}")]
    OperationFailed(#[from] anyhow::Error),
}

/// Domain errors related to GitHub operations.
#[derive(Debug, Error)]
pub enum GitHubError {
    #[error("Not authenticated with GitHub")]
    NotAuthenticated,

    #[error("GitHub API error: {0}")]
    ApiError(String),

    #[error("Invalid PR reference: owner={owner}, repo={repo}, number={number}")]
    InvalidPrRef {
        owner: String,
        repo: String,
        number: u32,
    },

    #[error("GitHub operation failed: {0}")]
    OperationFailed(#[from] anyhow::Error),
}

/// Domain errors related to diagram operations.
#[derive(Debug, Error)]
pub enum DiagramError {
    #[error("Invalid diagram format: {0}")]
    InvalidFormat(String),

    #[error("Diagram rendering failed: {0}")]
    RenderingFailed(String),

    #[error("Diagram operation failed: {0}")]
    OperationFailed(#[from] anyhow::Error),
}

/// Domain errors related to diff operations.
#[derive(Debug, Error)]
pub enum DiffError {
    #[error("Invalid diff format: {0}")]
    InvalidFormat(String),

    #[error("Diff position not found: file={file}, line={line}")]
    PositionNotFound { file: String, line: u32 },

    #[error("Diff operation failed: {0}")]
    OperationFailed(#[from] anyhow::Error),
}

/// Unified domain error type for application-level error handling.
///
/// This enum can be used when you need to handle multiple domain errors
/// in a unified way, or convert specific errors to anyhow for propagation.
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Review error: {0}")]
    Review(#[from] ReviewError),

    #[error("Task error: {0}")]
    Task(#[from] TaskError),

    #[error("Feedback error: {0}")]
    Feedback(#[from] FeedbackError),

    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),

    #[error("GitHub error: {0}")]
    GitHub(#[from] GitHubError),

    #[error("Diagram error: {0}")]
    Diagram(#[from] DiagramError),

    #[error("Diff error: {0}")]
    Diff(#[from] DiffError),

    #[error("Unknown domain error: {0}")]
    Unknown(String),
}

impl From<String> for DomainError {
    fn from(s: String) -> Self {
        DomainError::Unknown(s)
    }
}

impl From<&str> for DomainError {
    fn from(s: &str) -> Self {
        DomainError::Unknown(s.to_string())
    }
}
