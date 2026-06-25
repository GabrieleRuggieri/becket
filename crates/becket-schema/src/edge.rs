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
