//! Error types for the deterministic analysis core.

use thiserror::Error;

/// Errors raised during repository ingestion and analysis.
#[derive(Debug, Error)]
pub enum CoreError {
    /// Store or artifact persistence failure.
    #[error(transparent)]
    Store(#[from] repoctx_store::StoreError),

    /// Filesystem failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// File walker failure.
    #[error("walk error: {0}")]
    Walk(String),

    /// Invalid repository root or workspace configuration.
    #[error("invalid repository: {0}")]
    InvalidRepository(String),

    /// Tree-sitter parse failure.
    #[error("parse error: {0}")]
    Parse(String),

    /// Domain refinement error.
    #[error("domain error: {0}")]
    Domain(String),
}
