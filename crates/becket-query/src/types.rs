//! Structured query response types (CLI `--json` and MCP output).

use becket_schema::artifacts::{FlowRecord, SymbolRecord};
use serde::{Deserialize, Serialize};

/// Source of the optional enriched prose summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SummarySource {
    /// Template-based summary from indexed facts only.
    Deterministic,
    /// Host-delegated MCP sampling, cached in the index.
    McpSampling,
}

/// Task mode for context assembly ranking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ContextTask {
    /// Focused snippets, callers, tests, shallow impact.
    #[default]
    Fix,
    /// Deep impact, cross-module edges.
    Refactor,
    /// Flow overview, fewer snippets.
    Onboard,
}

impl ContextTask {
    /// Downstream impact traversal depth for this task.
    #[must_use]
    pub const fn impact_depth(self) -> u32 {
        match self {
            Self::Fix => 2,
            Self::Refactor => 5,
            Self::Onboard => 1,
        }
    }
}

/// A slice of real source code from disk.
#[derive(Debug, Clone, Serialize)]
pub struct CodeSnippet {
    /// Anchored symbol id.
    pub symbol_id: String,
    /// Symbol display name.
    pub symbol_name: String,
    /// Repository-relative file path.
    pub file_path: String,
    /// 1-based start line.
    pub start_line: u32,
    /// 1-based end line.
    pub end_line: u32,
    /// Fence language hint for markdown.
    pub language: String,
    /// Source lines (never model-generated).
    pub content: String,
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
    /// Real source snippets sliced from disk.
    pub snippets: Vec<CodeSnippet>,
    /// Direct caller symbol names.
    pub callers: Vec<String>,
    /// Direct callee symbol names.
    pub callees: Vec<String>,
    /// Downstream symbol ids within the task impact depth.
    pub affected_symbol_ids: Vec<String>,
    /// Grounded wiki page id when anchored to this symbol.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wiki_page_id: Option<String>,
    /// Wiki markdown body (without frontmatter) when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wiki_body: Option<String>,
    /// Agent-ready markdown bundle (one file).
    pub markdown: String,
    /// Task mode used for assembly.
    pub task: ContextTask,
    /// Token budget used for snippet packing.
    pub budget_tokens: u32,
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
