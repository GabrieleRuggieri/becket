//! Error types for the index store and artifact writer.

use thiserror::Error;

/// Errors raised by SQLite operations or artifact I/O.
#[derive(Debug, Error)]
pub enum StoreError {
    /// SQLite database error.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// Filesystem I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// A referenced entity was not found.
    #[error("not found: {0}")]
    NotFound(String),
}
