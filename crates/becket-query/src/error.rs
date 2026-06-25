//! Error types for query operations.

use thiserror::Error;

/// Errors raised when resolving symbols or running graph queries.
#[derive(Debug, Error)]
pub enum QueryError {
    /// Underlying store failure.
    #[error(transparent)]
    Store(#[from] becket_store::StoreError),

    /// No index found — run `becket build` first.
    #[error("index not found at {0}; run `becket build` first")]
    IndexMissing(String),

    /// Symbol or flow not found.
    #[error("not found: {0}")]
    NotFound(String),
}
