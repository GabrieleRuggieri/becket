//! Graph edge types and cross-service boundary markers.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Typed relationship between two symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    /// Function or method invocation.
    Calls,
    /// Module or package import.
    Imports,
    /// Class inheritance.
    Extends,
    /// Interface or trait implementation.
    Implements,
    /// Generic reference without a stronger kind.
    References,
    /// Read access to a symbol.
    Reads,
    /// Write access to a symbol.
    Writes,
    /// HTTP client to server edge.
    Http,
    /// gRPC client to server edge.
    Grpc,
    /// Message queue producer to consumer edge.
    Queue,
}

/// How an edge target was resolved, from strongest to weakest evidence.
///
/// Determines the edge `confidence` and lets consumers (ranking, prompts)
/// distinguish type-resolved facts from syntactic guesses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EdgeResolution {
    /// Resolved by a semantic indexer (SCIP/LSP deep mode).
    TypeResolved,
    /// Resolved through an import statement to a specific file.
    ImportResolved,
    /// Target declared in the same file as the source.
    FileScoped,
    /// Unique (or unique public) match in the same directory.
    DirScoped,
    /// Single match across the whole repository.
    GlobalUnique,
    /// One of several same-name candidates (ambiguous syntactic match).
    Candidate,
}

impl EdgeResolution {
    /// Confidence score associated with this resolution tier.
    #[must_use]
    pub const fn confidence(self) -> f32 {
        match self {
            Self::TypeResolved => 1.0,
            Self::ImportResolved => 0.9,
            Self::FileScoped => 0.85,
            Self::DirScoped => 0.7,
            Self::GlobalUnique => 0.5,
            Self::Candidate => 0.25,
        }
    }

    /// Human-readable label for prompts and reports.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::TypeResolved => "type-resolved",
            Self::ImportResolved => "import-resolved",
            Self::FileScoped => "same-file",
            Self::DirScoped => "same-dir",
            Self::GlobalUnique => "global-unique",
            Self::Candidate => "candidate",
        }
    }

    /// Canonical string id stored in the index and artifacts.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TypeResolved => "type_resolved",
            Self::ImportResolved => "import_resolved",
            Self::FileScoped => "file_scoped",
            Self::DirScoped => "dir_scoped",
            Self::GlobalUnique => "global_unique",
            Self::Candidate => "candidate",
        }
    }

    /// Parses the canonical string id (defaults to `FileScoped` for legacy rows).
    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value {
            "type_resolved" => Self::TypeResolved,
            "import_resolved" => Self::ImportResolved,
            "dir_scoped" => Self::DirScoped,
            "global_unique" => Self::GlobalUnique,
            "candidate" => Self::Candidate,
            _ => Self::FileScoped,
        }
    }
}

/// Boundary crossed by a cross-repo or cross-service edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryKind {
    /// Network call across service boundary.
    Network,
    /// Message queue boundary.
    Queue,
    /// Shared library dependency across repos.
    SharedLib,
}
