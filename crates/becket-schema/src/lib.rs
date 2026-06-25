//! Versioned JSON artifact schemas and shared domain types for Becket.
//!
//! This crate defines the public contract for `.becket/*.json` artifacts and
//! shared enums used across the deterministic core, store, query engine, and CLI.

pub mod artifacts;
pub mod edge;
pub mod error;
pub mod json_schema;
pub mod symbol;
pub mod version;
pub mod wiki;
pub mod workspace;

pub use artifacts::{
    ArchitectureArtifact, DependenciesArtifact, EntrypointsArtifact, FlowsArtifact,
    SymbolsArtifact, VersionedArtifact,
};
pub use edge::{BoundaryKind, EdgeType};
pub use error::SchemaError;
pub use json_schema::{
    parse_artifact, pretty_schema_for, root_schema_for, validate_artifact_json, ARTIFACT_NAMES,
};
pub use symbol::{EntrypointKind, SymbolKind, Visibility};
pub use version::SCHEMA_VERSION;
pub use wiki::{
    WikiClaimError, WikiLinkError, WikiLintArtifact, WikiPage, WikiPageKind, WikiPageMeta,
    WikiPageSource, WikiStaleQueue,
};
pub use workspace::{
    CrossRepoArtifact, CrossRepoEdgeRecord, HttpContract, SharedLibContract, WorkspaceContracts,
    WorkspaceManifest, WorkspaceRepo,
};
