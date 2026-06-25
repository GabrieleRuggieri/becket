//! Wiki knowledge layer types (`.becket/wiki/` + lint artifacts).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::version::SCHEMA_VERSION;

/// Page taxonomy derived from the code graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WikiPageKind {
    /// Router / table of contents.
    Overview,
    /// Logical module from architecture graph.
    Module,
    /// Service or handler anchored to entrypoints.
    Service,
    /// Business flow from flow reconstructor.
    Flow,
    /// Cross-cutting concept (reserved).
    Concept,
}

/// How the page body was authored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WikiPageSource {
    /// Compiled from the deterministic graph only.
    Deterministic,
    /// Prose slot enriched via MCP sampling.
    McpSampling,
}

/// Parsed wiki page frontmatter (TOML between `---` fences).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct WikiPageMeta {
    /// Stable page id (e.g. `wiki_flow_payment`).
    pub id: String,
    /// Page taxonomy.
    pub kind: WikiPageKind,
    /// Anchored symbol ids from the graph.
    pub symbol_ids: Vec<String>,
    /// Authoring source.
    pub source: WikiPageSource,
    /// Fingerprint of the anchored subgraph at compile time.
    pub graph_fingerprint: String,
    /// Related wiki page ids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub see_also: Vec<String>,
    /// Human title for index routing.
    pub title: String,
}

/// A loaded wiki page (metadata + markdown body).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct WikiPage {
    pub meta: WikiPageMeta,
    /// Markdown body without frontmatter.
    pub body: String,
    /// True when `graph_fingerprint` no longer matches the live graph.
    pub stale: bool,
}

/// `wiki_lint.json` — deterministic lint report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WikiLintArtifact {
    pub schema_version: String,
    pub stale_page_ids: Vec<String>,
    pub claim_errors: Vec<WikiClaimError>,
    pub broken_links: Vec<WikiLinkError>,
    pub orphan_page_ids: Vec<String>,
}

impl Default for WikiLintArtifact {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            stale_page_ids: Vec::new(),
            claim_errors: Vec::new(),
            broken_links: Vec::new(),
            orphan_page_ids: Vec::new(),
        }
    }
}

/// A machine-readable claim that failed verification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct WikiClaimError {
    pub page_id: String,
    pub claim: String,
    pub message: String,
}

/// A broken `see_also` or inline wiki link.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct WikiLinkError {
    pub page_id: String,
    pub target: String,
    pub message: String,
}

/// `.becket/wiki_stale.json` — pages queued for re-sync after watch/build.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WikiStaleQueue {
    pub schema_version: String,
    pub page_ids: Vec<String>,
}

impl Default for WikiStaleQueue {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            page_ids: Vec::new(),
        }
    }
}
