//! Query engine implementation backed by the SQLite index.

use std::path::Path;

use repoctx_embed::{embed_with_model, symbol_embedding_text};
use repoctx_schema::artifacts::SymbolRecord;
use repoctx_store::{IndexStore, RepoCtxPaths};

use crate::error::QueryError;
use crate::types::{ContextResult, DependenciesResult, FlowResult, ImpactResult, SummarySource};

/// Read-only query surface over a built `.repoctx/` index.
pub struct QueryEngine {
    paths: RepoCtxPaths,
}

impl QueryEngine {
    /// Opens the query engine for the repository at `root`.
    ///
    /// # Arguments
    ///
    /// * `root` - Repository root directory.
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            paths: RepoCtxPaths::new(root),
        }
    }

    /// Returns an opened index store or a helpful error.
    fn open_store(&self) -> Result<IndexStore, QueryError> {
        if !self.paths.index_db.exists() {
            return Err(QueryError::IndexMissing(
                self.paths.index_db.display().to_string(),
            ));
        }
        Ok(IndexStore::open(&self.paths.index_db)?)
    }

    /// Resolves a symbol by name or FQN.
    fn resolve_symbol(&self, store: &IndexStore, query: &str) -> Result<SymbolRecord, QueryError> {
        let matches = store.find_symbols_by_name(query)?;
        matches
            .into_iter()
            .next()
            .ok_or_else(|| QueryError::NotFound(format!("symbol '{query}'")))
    }

    /// Computes downstream impact for `symbol` within `depth` hops.
    ///
    /// # Errors
    ///
    /// Returns [`QueryError`] when the index is missing or symbol is unknown.
    pub fn impact(&self, symbol: &str, depth: u32) -> Result<ImpactResult, QueryError> {
        let store = self.open_store()?;
        let root = self.resolve_symbol(&store, symbol)?;
        let affected_ids = store.downstream_symbols(&root.id, depth)?;

        let all_symbols = store.load_symbols()?;
        let id_to_symbol: std::collections::HashMap<_, _> =
            all_symbols.iter().map(|s| (s.id.as_str(), s)).collect();

        let mut affected_modules = Vec::new();
        let mut related_tests = Vec::new();
        let mut risk_zones = Vec::new();

        for id in &affected_ids {
            if let Some(sym) = id_to_symbol.get(id.as_str()) {
                let module = sym.file_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
                if !module.is_empty() && !affected_modules.contains(&module.to_string()) {
                    affected_modules.push(module.to_string());
                }
                if sym.file_path.contains("test") || sym.file_path.contains("spec") {
                    related_tests.push(sym.file_path.clone());
                }
            }
        }

        if affected_ids.len() > 10 {
            risk_zones.push(format!(
                "{} has {} downstream dependents within depth {}",
                root.name,
                affected_ids.len(),
                depth
            ));
        }

        Ok(ImpactResult {
            symbol: root,
            affected_symbol_ids: affected_ids,
            affected_modules,
            related_tests,
            risk_zones,
        })
    }

    /// Looks up a business flow by domain name.
    ///
    /// # Errors
    ///
    /// Returns [`QueryError`] when the index is missing.
    pub fn flow(&self, domain: &str) -> Result<FlowResult, QueryError> {
        let store = self.open_store()?;
        let flow = store.find_flow_by_name(domain)?;

        let suggestions = if flow.is_none() {
            store
                .load_symbols()?
                .into_iter()
                .filter(|s| s.name.to_lowercase().contains(&domain.to_lowercase()))
                .map(|s| s.name)
                .take(5)
                .collect()
        } else {
            Vec::new()
        };

        Ok(FlowResult {
            flow,
            suggestions,
            enriched_description: None,
            description_source: SummarySource::Deterministic,
        })
    }

    /// Builds a compact context bundle for LLM consumption.
    ///
    /// # Errors
    ///
    /// Returns [`QueryError`] when the index is missing or symbol is unknown.
    pub fn context(&self, symbol: &str, _budget: Option<u32>) -> Result<ContextResult, QueryError> {
        let store = self.open_store()?;
        let root = self.resolve_symbol(&store, symbol)?;

        let all_symbols = store.load_symbols()?;
        let related_components: Vec<String> = all_symbols
            .iter()
            .filter(|s| s.file_path == root.file_path && s.id != root.id)
            .map(|s| s.name.clone())
            .take(10)
            .collect();

        let responsibility = format!(
            "{} ({:?}) defined in {} at lines {}-{}",
            root.name, root.kind, root.file_path, root.start_line, root.end_line
        );

        let external_dependencies = root
            .file_path
            .split('/')
            .take(2)
            .map(str::to_string)
            .collect();

        let invariants = vec![format!("visibility: {:?}", root.visibility)];

        let semantic_neighbors = self
            .semantic_neighbor_names(&store, &root, 5)
            .unwrap_or_default();

        Ok(ContextResult {
            symbol: root,
            responsibility,
            enriched_summary: None,
            summary_source: SummarySource::Deterministic,
            related_components,
            external_dependencies,
            invariants,
            semantic_neighbors,
        })
    }

    fn semantic_neighbor_names(
        &self,
        store: &IndexStore,
        symbol: &SymbolRecord,
        limit: usize,
    ) -> Result<Vec<String>, QueryError> {
        if store.count_symbol_embeddings()? == 0 {
            return Ok(Vec::new());
        }
        let query = embed_with_model(&symbol_embedding_text(symbol));
        let hits = store.nearest_symbol_ids(&query, limit + 1)?;
        if hits.is_empty() {
            return Ok(Vec::new());
        }
        let id_to_name: std::collections::HashMap<_, _> = store
            .load_symbols()?
            .into_iter()
            .map(|s| (s.id, s.name))
            .collect();
        Ok(hits
            .into_iter()
            .filter(|(id, _)| id != &symbol.id)
            .take(limit)
            .filter_map(|(id, _)| id_to_name.get(&id).cloned())
            .collect())
    }

    /// Lists downstream dependencies for a symbol.
    ///
    /// # Errors
    ///
    /// Returns [`QueryError`] when the index is missing or symbol is unknown.
    pub fn dependencies(&self, symbol: &str, depth: u32) -> Result<DependenciesResult, QueryError> {
        let store = self.open_store()?;
        let root = self.resolve_symbol(&store, symbol)?;
        let downstream = store.downstream_symbols(&root.id, depth)?;

        Ok(DependenciesResult {
            symbol: root,
            downstream,
            upstream: Vec::new(),
        })
    }
}
