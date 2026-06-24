//! Deterministic analysis core for RepoCtx.
//!
//! Walks the repository, hashes files for incremental builds, extracts symbols
//! with language heuristics (tree-sitter integration planned), and orchestrates
//! the `repoctx build` pipeline.

pub mod build;
pub mod domain;
pub mod embed;
pub mod error;
pub mod extract;
pub mod flow;
pub mod graph;
pub mod ids;
pub mod language;
pub mod parse;
pub mod redact;
pub mod walker;
pub mod workspace;

pub use flow::FlowReconstructor;
pub use ids::{
    stable_cross_repo_edge_id, stable_edge_id, stable_entrypoint_id, stable_file_id,
    stable_flow_id, stable_symbol_id,
};

pub use build::{BuildOptions, BuildPipeline, BuildReport};
pub use domain::DomainEditor;
pub use error::CoreError;
pub use redact::redact_secrets;
pub use workspace::{
    discover_workspace_manifest, is_workspace_root, load_workspace_manifest, WorkspaceBuildReport,
    WorkspacePipeline, WORKSPACE_MANIFEST_FILE,
};
