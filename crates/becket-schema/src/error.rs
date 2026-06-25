//! Error types for schema validation and serialization.

use thiserror::Error;

/// Errors raised when validating or serializing artifact schemas.
#[derive(Debug, Error)]
pub enum SchemaError {
    /// JSON serialization or deserialization failed.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// A required field was missing or invalid.
    #[error("invalid schema field: {0}")]
    InvalidField(String),

    /// The artifact schema version is unsupported.
    #[error("unsupported schema version: {0}")]
    UnsupportedVersion(String),

    /// Unknown artifact name.
    #[error("unknown artifact: {0}")]
    UnknownArtifact(String),

    /// JSON instance failed to parse.
    #[error("invalid JSON for {artifact}: {source}")]
    InvalidJson {
        /// Artifact base name.
        artifact: String,
        /// Underlying parse error.
        source: serde_json::Error,
    },

    /// JSON Schema document is malformed.
    #[error("invalid schema for {artifact}: {message}")]
    InvalidSchema {
        /// Artifact base name.
        artifact: String,
        /// Validator construction error.
        message: String,
    },

    /// Instance failed schema validation.
    #[error("schema validation failed for {artifact}: {messages:?}")]
    ValidationFailed {
        /// Artifact base name.
        artifact: String,
        /// Validation error messages.
        messages: Vec<String>,
    },
}
