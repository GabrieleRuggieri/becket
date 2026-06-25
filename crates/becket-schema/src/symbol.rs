//! Symbol-related enums shared across artifacts and the index store.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Kind of symbol extracted from source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    /// Top-level or nested function.
    Function,
    /// Class, struct, or interface type.
    Class,
    /// Method on a class or struct.
    Method,
    /// Variable or constant binding.
    Var,
    /// Named type alias or typedef.
    Type,
    /// Module or package boundary.
    Module,
}

/// Visibility of a symbol within its scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// Public or exported symbol.
    Public,
    /// Package-private or internal symbol.
    Internal,
    /// Private to enclosing scope.
    Private,
}

/// Kind of application entry point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EntrypointKind {
    /// Command-line interface entry.
    Cli,
    /// HTTP route or handler.
    Http,
    /// Scheduled cron job.
    Cron,
    /// Event or message consumer.
    Event,
    /// `main` or program entry.
    Main,
}
