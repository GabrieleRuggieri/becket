//! Versioned JSON artifact document types emitted under `.becket/`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::edge::{BoundaryKind, EdgeType};
use crate::symbol::{EntrypointKind, SymbolKind, Visibility};
use crate::version::SCHEMA_VERSION;

/// Wrapper trait for artifacts that embed `schemaVersion`.
pub trait VersionedArtifact {
    /// Returns the embedded schema version string.
    fn schema_version(&self) -> &str;
}

/// High-level structural map (`architecture.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ArchitectureArtifact {
    /// SemVer schema version for this artifact.
    pub schema_version: String,
    /// Logical modules discovered in the repository.
    pub modules: Vec<ModuleRecord>,
    /// Summary edges between modules.
    pub edges: Vec<ModuleEdgeRecord>,
}

impl Default for ArchitectureArtifact {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            modules: Vec::new(),
            edges: Vec::new(),
        }
    }
}

impl VersionedArtifact for ArchitectureArtifact {
    fn schema_version(&self) -> &str {
        &self.schema_version
    }
}

/// A logical module grouping symbols.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct ModuleRecord {
    /// Stable module identifier.
    pub id: String,
    /// Human-readable module name.
    pub name: String,
    /// Module classification.
    pub kind: String,
    /// Symbol IDs grouped under this module.
    pub symbol_ids: Vec<String>,
}

/// Directed edge between two modules.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct ModuleEdgeRecord {
    /// Source module id.
    pub from: String,
    /// Target module id.
    pub to: String,
    /// Edge classification.
    pub edge_type: EdgeType,
}

/// Symbol catalog (`symbols.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SymbolsArtifact {
    /// SemVer schema version for this artifact.
    pub schema_version: String,
    /// All symbols indexed in the repository.
    pub symbols: Vec<SymbolRecord>,
}

impl Default for SymbolsArtifact {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            symbols: Vec::new(),
        }
    }
}

impl VersionedArtifact for SymbolsArtifact {
    fn schema_version(&self) -> &str {
        &self.schema_version
    }
}

/// A single indexed symbol with source location.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SymbolRecord {
    /// Stable symbol identifier.
    pub id: String,
    /// Symbol kind.
    pub kind: SymbolKind,
    /// Short name.
    pub name: String,
    /// Fully qualified name.
    pub fqn: String,
    /// Repository-relative file path.
    pub file_path: String,
    /// 1-based start line.
    pub start_line: u32,
    /// 1-based end line.
    pub end_line: u32,
    /// Visibility within scope.
    pub visibility: Visibility,
    /// Optional module id this symbol belongs to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_id: Option<String>,
}

/// Dependency graph (`dependencies.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DependenciesArtifact {
    /// SemVer schema version for this artifact.
    pub schema_version: String,
    /// Directed edges between symbols.
    pub edges: Vec<DependencyEdgeRecord>,
}

impl Default for DependenciesArtifact {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            edges: Vec::new(),
        }
    }
}

impl VersionedArtifact for DependenciesArtifact {
    fn schema_version(&self) -> &str {
        &self.schema_version
    }
}

/// A dependency or call edge between two symbols.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DependencyEdgeRecord {
    /// Stable edge identifier.
    pub id: String,
    /// Source symbol id.
    pub src_symbol_id: String,
    /// Target symbol id.
    pub dst_symbol_id: String,
    /// Edge type.
    pub edge_type: EdgeType,
    /// Cross-service boundary, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boundary: Option<BoundaryKind>,
    /// Confidence score (1.0 = static certainty).
    pub confidence: f32,
}

/// Reconstructed business flows (`flows.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FlowsArtifact {
    /// SemVer schema version for this artifact.
    pub schema_version: String,
    /// Discovered or configured flows.
    pub flows: Vec<FlowRecord>,
}

impl Default for FlowsArtifact {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            flows: Vec::new(),
        }
    }
}

impl VersionedArtifact for FlowsArtifact {
    fn schema_version(&self) -> &str {
        &self.schema_version
    }
}

/// A named execution flow spanning one or more symbols.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct FlowRecord {
    /// Stable flow identifier.
    pub id: String,
    /// Flow name (e.g. `payment`).
    pub name: String,
    /// Human-readable description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Ordered steps in the flow.
    pub steps: Vec<FlowStepRecord>,
}

/// One step in a reconstructed flow.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct FlowStepRecord {
    /// Step order (0-based).
    pub order: u32,
    /// Referenced symbol id.
    pub symbol_id: String,
    /// External system involved at this step, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_system: Option<String>,
}

/// Detected entry points (`entrypoints.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EntrypointsArtifact {
    /// SemVer schema version for this artifact.
    pub schema_version: String,
    /// Entry points indexed in the repository.
    pub entrypoints: Vec<EntrypointRecord>,
}

impl Default for EntrypointsArtifact {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            entrypoints: Vec::new(),
        }
    }
}

impl VersionedArtifact for EntrypointsArtifact {
    fn schema_version(&self) -> &str {
        &self.schema_version
    }
}

/// A detected program entry point.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct EntrypointRecord {
    /// Stable entrypoint identifier.
    pub id: String,
    /// Symbol id for the entry point.
    pub symbol_id: String,
    /// Entry point classification.
    pub kind: EntrypointKind,
    /// Optional human label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}
