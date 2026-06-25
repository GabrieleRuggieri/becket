//! Writes versioned JSON artifacts to `.becket/`.

use std::fs;
use std::path::Path;

use becket_schema::VersionedArtifact;
use serde::Serialize;

use crate::error::StoreError;
use crate::paths::BecketPaths;

/// Serializes artifact documents to `.becket/*.json`.
pub struct ArtifactWriter {
    paths: BecketPaths,
}

impl ArtifactWriter {
    /// Creates a writer bound to the repository's `.becket/` directory.
    ///
    /// # Arguments
    ///
    /// * `paths` - Resolved Becket output paths.
    pub fn new(paths: BecketPaths) -> Self {
        Self { paths }
    }

    /// Ensures `.becket/` exists on disk.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Io`] if directory creation fails.
    pub fn ensure_output_dir(&self) -> Result<(), StoreError> {
        fs::create_dir_all(&self.paths.output_dir)?;
        Ok(())
    }

    /// Writes a single artifact as pretty-printed JSON.
    ///
    /// # Arguments
    ///
    /// * `filename` - Base name without extension.
    /// * `artifact` - Serializable artifact implementing [`VersionedArtifact`].
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Json`] or [`StoreError::Io`] on failure.
    pub fn write_artifact<T>(&self, filename: &str, artifact: &T) -> Result<(), StoreError>
    where
        T: Serialize + VersionedArtifact,
    {
        self.ensure_output_dir()?;
        let path = self.paths.artifact(filename);
        let json = serde_json::to_string_pretty(artifact)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Returns the output directory path.
    pub fn output_dir(&self) -> &Path {
        &self.paths.output_dir
    }
}
