//! Query engine implementation backed by the SQLite index.

use std::path::Path;

use becket_schema::artifacts::SymbolRecord;
use becket_store::{IndexStore, BecketPaths};

use crate::assemble::assemble_context;
use crate::error::QueryError;
use crate::types::{
    ContextResult, ContextTask, DependenciesResult, FlowResult, ImpactResult, SummarySource,
};

/// Read-only query surface over a built `.becket/` index.
pub struct QueryEngine {
    paths: BecketPaths,
}

impl QueryEngine {
    /// Opens the query engine for the repository at `root`.
    ///
    /// # Arguments
    ///
    /// * `root` - Repository root directory.
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            paths: BecketPaths::new(root),
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

    /// Builds a context bundle with code snippets, impact, and markdown for LLM consumption.
    ///
    /// # Errors
    ///
    /// Returns [`QueryError`] when the index is missing or symbol is unknown.
    pub fn context(
        &self,
        symbol: &str,
        budget: Option<u32>,
        task: ContextTask,
    ) -> Result<ContextResult, QueryError> {
        let store = self.open_store()?;
        let root = self.resolve_symbol(&store, symbol)?;
        assemble_context(&store, &self.paths.root, root, budget, task)
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
