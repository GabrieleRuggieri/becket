//! Query engine for impact, flow, context, and dependency lookups.
//!
//! Shared by the CLI and MCP server — no duplicated query logic.

pub mod assemble;
pub mod bench;
pub mod budget;
pub mod engine;
pub mod error;
pub mod report;
pub mod types;

pub use assemble::{
    assemble_context, assemble_context_with_options, refresh_context_markdown, AssembleOptions,
};
pub use bench::{run_bench, BenchReport};
pub use engine::QueryEngine;
pub use error::QueryError;
pub use report::{generate_report, ReportMetrics};
pub use types::{
    BudgetAdvice, CodeSnippet, ContextResult, ContextTask, DependenciesResult, FlowResult,
    ImpactResult, SummarySource,
};
