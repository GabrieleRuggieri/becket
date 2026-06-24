//! SQLite index and JSON artifact persistence for RepoCtx.
//!
//! Owns the rebuildable `.repoctx/index.db` cache and emits versioned JSON
//! artifacts under `.repoctx/*.json`.

pub mod artifacts;
pub mod db;
pub mod error;
pub mod paths;

pub use artifacts::ArtifactWriter;
pub use db::{DomainOverride, IndexStore};
pub use error::StoreError;
pub use paths::RepoCtxPaths;
