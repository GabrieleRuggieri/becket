//! Multi-repo workspace manifest and cross-repo artifact types.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::edge::{BoundaryKind, EdgeType};
use crate::version::SCHEMA_VERSION;

/// Workspace manifest (`repoctx.workspace.toml`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceManifest {
    /// Schema version for the manifest file.
    pub schema_version: String,
    /// Human-readable workspace name.
    pub name: String,
    /// Member repositories.
    pub repos: Vec<WorkspaceRepo>,
    /// Optional explicit service contracts.
    #[serde(default)]
    pub contracts: WorkspaceContracts,
}

impl Default for WorkspaceManifest {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            name: String::new(),
            repos: Vec::new(),
            contracts: WorkspaceContracts::default(),
        }
    }
}

/// A repository member of a workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceRepo {
    /// Short repo name (unique within the workspace).
    pub name: String,
    /// Path relative to the workspace root.
    pub path: String,
}

/// Optional explicit cross-repo contracts.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceContracts {
    /// HTTP client ↔ server contracts.
    #[serde(default)]
    pub http: Vec<HttpContract>,
    /// Shared library package contracts.
    #[serde(default)]
    pub shared_lib: Vec<SharedLibContract>,
}

/// Explicit HTTP route contract between two repos.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HttpContract {
    /// Repo that issues outbound HTTP calls.
    pub client_repo: String,
    /// Repo that exposes the HTTP route.
    pub server_repo: String,
    /// HTTP method (GET, POST, …).
    pub method: String,
    /// Route path (e.g. `/users`).
    pub path: String,
}

/// Shared library contract matched by import/package name.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SharedLibContract {
    /// Package or module name shared across repos.
    pub package: String,
    /// Repos that participate in the contract.
    pub repos: Vec<String>,
}

/// Cross-repo edges artifact (`cross_repo.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrossRepoArtifact {
    /// SemVer schema version for this artifact.
    pub schema_version: String,
    /// Workspace name.
    pub workspace: String,
    /// Cross-repository edges.
    pub edges: Vec<CrossRepoEdgeRecord>,
}

impl Default for CrossRepoArtifact {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            workspace: String::new(),
            edges: Vec::new(),
        }
    }
}

/// A directed edge between symbols in different repositories.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrossRepoEdgeRecord {
    /// Stable edge identifier.
    pub id: String,
    /// Source repository name.
    pub src_repo: String,
    /// Source symbol id.
    pub src_symbol_id: String,
    /// Target repository name.
    pub dst_repo: String,
    /// Target symbol id.
    pub dst_symbol_id: String,
    /// Edge type.
    pub edge_type: EdgeType,
    /// Cross-service boundary.
    pub boundary: BoundaryKind,
    /// Confidence score (1.0 = explicit contract).
    pub confidence: f32,
    /// Optional human label (route, package name, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}
