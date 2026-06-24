//! Query engine for impact, flow, context, and dependency lookups.
//!
//! Shared by the CLI and MCP server — no duplicated query logic.

pub mod assemble;
pub mod engine;
pub mod error;
pub mod types;

pub use engine::QueryEngine;
pub use error::QueryError;
pub use types::{
    CodeSnippet, ContextResult, ContextTask, DependenciesResult, FlowResult, ImpactResult,
    SummarySource,
};
