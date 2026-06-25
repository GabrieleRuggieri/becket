//! SQLite index and JSON artifact persistence for Becket.
//!
//! Owns the rebuildable `.becket/index.db` cache and emits versioned JSON
//! artifacts under `.becket/*.json`.

pub mod artifacts;
pub mod db;
pub mod error;
pub mod paths;
pub mod vec_ext;

pub use artifacts::ArtifactWriter;
pub use db::{DomainOverride, EnrichmentRecord, IndexStore};
pub use error::StoreError;
pub use paths::BecketPaths;
pub use vec_ext::ensure_sqlite_vec;
