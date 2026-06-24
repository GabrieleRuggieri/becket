//! Structured query response types (CLI `--json` and MCP output).

use repoctx_schema::artifacts::{FlowRecord, SymbolRecord};
use serde::Serialize;

/// Source of the optional enriched prose summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SummarySource {
    /// Template-based summary from indexed facts only.
    Deterministic,
    /// Host-delegated MCP sampling, cached in the index.
    McpSampling,
}

/// Impact analysis result for a symbol.
#[derive(Debug, Clone, Serialize)]
pub struct ImpactResult {
    /// Resolved root symbol.
    pub symbol: SymbolRecord,
    /// Downstream symbol ids within the requested depth.
    pub affected_symbol_ids: Vec<String>,
    /// Human-readable module paths at risk.
    pub affected_modules: Vec<String>,
    /// Related test file paths (heuristic: `*test*` in path).
    pub related_tests: Vec<String>,
    /// Symbols with many downstream dependents.
    pub risk_zones: Vec<String>,
}

/// Flow lookup result for a domain name.
#[derive(Debug, Clone, Serialize)]
pub struct FlowResult {
    /// Matched flow record, if any.
    pub flow: Option<FlowRecord>,
    /// Suggested domain names when no exact match exists.
    pub suggestions: Vec<String>,
    /// LLM-enriched flow description when available via MCP sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enriched_description: Option<String>,
    /// Whether `enriched_description` came from cache/sampling or is absent.
    pub description_source: SummarySource,
}

/// LLM-oriented context bundle for a symbol.
#[derive(Debug, Clone, Serialize)]
pub struct ContextResult {
    /// Resolved symbol.
    pub symbol: SymbolRecord,
    /// Deterministic responsibility summary.
    pub responsibility: String,
    /// Optional host-enriched prose (lazy MCP sampling).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enriched_summary: Option<String>,
    /// Source of `enriched_summary` when present.
    pub summary_source: SummarySource,
    /// Related symbol names in the same file or module.
    pub related_components: Vec<String>,
    /// External dependency hints (from file path segments).
    pub external_dependencies: Vec<String>,
    /// Static invariants inferred from visibility and kind.
    pub invariants: Vec<String>,
    /// Semantically similar symbol names (sqlite-vec), when embeddings are indexed.
    pub semantic_neighbors: Vec<String>,
}

/// Direct and transitive dependency listing.
#[derive(Debug, Clone, Serialize)]
pub struct DependenciesResult {
    /// Resolved root symbol.
    pub symbol: SymbolRecord,
    /// Downstream symbol ids.
    pub downstream: Vec<String>,
    /// Upstream symbol ids (not yet populated in v0).
    pub upstream: Vec<String>,
}
