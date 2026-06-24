//! `repoctx build` orchestration: walk, hash, extract, persist, emit artifacts.

use std::collections::HashMap;
use std::path::Path;

use repoctx_schema::artifacts::EntrypointRecord;
use repoctx_schema::edge::EdgeType;
use repoctx_schema::symbol::EntrypointKind;
use repoctx_store::{ArtifactWriter, IndexStore, RepoCtxPaths};
use tracing::info;

use crate::domain::apply_domain_overrides;
use crate::error::CoreError;
use crate::flow::{CallEdge, FlowReconstructor};
use crate::graph::GraphResolver;
use crate::ids::{stable_entrypoint_id, stable_file_id};
use crate::parse::{FileParseResult, ParsedCall, TreeSitterParser};
use crate::walker::{FileWalker, SourceFile};

/// Options controlling a build run.
#[derive(Debug, Clone)]
pub struct BuildOptions {
    /// When true, skip files whose content hash is unchanged.
    pub incremental: bool,
    /// When true, skip embedding generation (not yet implemented).
    pub no_embeddings: bool,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            incremental: true,
            no_embeddings: false,
        }
    }
}

/// Summary counters emitted after a successful build.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BuildReport {
    /// Total source files discovered.
    pub files_discovered: usize,
    /// Files parsed in this run.
    pub files_parsed: usize,
    /// Files skipped due to incremental cache hit.
    pub files_skipped: usize,
    /// Symbols indexed.
    pub symbols_indexed: usize,
    /// Dependency edges resolved.
    pub edges_indexed: usize,
    /// Entrypoints detected.
    pub entrypoints_indexed: usize,
    /// Flows auto-discovered.
    pub flows_indexed: usize,
    /// Path to `.repoctx/` output directory.
    pub output_dir: String,
}

/// End-to-end deterministic build pipeline.
pub struct BuildPipeline {
    paths: RepoCtxPaths,
    options: BuildOptions,
}

impl BuildPipeline {
    /// Creates a pipeline for the repository at `root`.
    pub fn new(root: impl AsRef<Path>, options: BuildOptions) -> Self {
        Self {
            paths: RepoCtxPaths::new(root),
            options,
        }
    }

    /// Runs the full build: walk → parse → index → emit JSON artifacts.
    pub fn run(&self) -> Result<BuildReport, CoreError> {
        let walker = FileWalker::new(&self.paths.root);
        let discovered = walker.discover()?;

        let store = IndexStore::open(&self.paths.index_db)?;
        if !self.options.incremental {
            store.clear_all()?;
        }

        let mut files_parsed = 0usize;
        let mut files_skipped = 0usize;
        let mut symbols_indexed = 0usize;
        let mut parse_cache: HashMap<String, FileParseResult> = HashMap::new();

        for file in &discovered {
            let parsed = TreeSitterParser::parse_file(
                &file.relative_path,
                file.language,
                &file.absolute_path,
            )?;
            parse_cache.insert(file.relative_path.clone(), parsed);

            if self.should_skip_file(&store, file)? {
                files_skipped += 1;
                continue;
            }

            let parsed = parse_cache.get(&file.relative_path).expect("just inserted");

            store.delete_symbols_for_path(&file.relative_path)?;

            let file_id = stable_file_id(&file.relative_path);

            store.upsert_file(
                &file_id,
                &file.relative_path,
                file.language.id(),
                &file.content_hash,
                file.mtime_secs,
            )?;

            for symbol in &parsed.symbols {
                store.insert_symbol(symbol, &file_id)?;
                symbols_indexed += 1;
            }

            files_parsed += 1;
            info!(path = %file.relative_path, symbols = parsed.symbols.len(), "parsed file");
        }

        let all_symbols = store.load_symbols()?;
        let all_calls = self.remap_calls(&parse_cache, &all_symbols);
        let all_imports = collect_imports(&parse_cache);

        let edges = GraphResolver::resolve(&all_symbols, &all_calls, &all_imports);
        store.clear_edges()?;
        for edge in &edges {
            store.insert_edge(edge)?;
        }

        store.clear_entrypoints()?;
        let entrypoints_indexed = self.index_entrypoints(&store, &all_symbols)?;

        let call_edges: Vec<CallEdge> = edges
            .iter()
            .filter(|e| e.edge_type == EdgeType::Calls)
            .map(|e| CallEdge {
                src: e.src_symbol_id.clone(),
                dst: e.dst_symbol_id.clone(),
            })
            .collect();
        let discovered_flows = {
            let mut flows = FlowReconstructor::reconstruct(&all_symbols, &call_edges);
            let overrides = store.load_user_domain_overrides()?;
            apply_domain_overrides(&mut flows, &overrides, &all_symbols, &call_edges);
            flows.sort_by(|a, b| a.name.cmp(&b.name));
            flows
        };
        store.clear_flows()?;
        for flow in &discovered_flows {
            store.insert_flow(flow)?;
        }
        store.sync_domains_from_flows(&discovered_flows)?;
        let flows_indexed = discovered_flows.len();

        let writer = ArtifactWriter::new(self.paths.clone());
        let (symbols, dependencies, flows, entrypoints, architecture) = store.export_artifacts()?;

        writer.write_artifact("symbols", &symbols)?;
        writer.write_artifact("dependencies", &dependencies)?;
        writer.write_artifact("flows", &flows)?;
        writer.write_artifact("entrypoints", &entrypoints)?;
        writer.write_artifact("architecture", &architecture)?;

        Ok(BuildReport {
            files_discovered: discovered.len(),
            files_parsed,
            files_skipped,
            symbols_indexed,
            edges_indexed: edges.len(),
            entrypoints_indexed,
            flows_indexed,
            output_dir: writer.output_dir().display().to_string(),
        })
    }

    fn should_skip_file(&self, store: &IndexStore, file: &SourceFile) -> Result<bool, CoreError> {
        if !self.options.incremental {
            return Ok(false);
        }
        if let Some(existing_hash) = store.file_hash(&file.relative_path)? {
            return Ok(existing_hash == file.content_hash);
        }
        Ok(false)
    }

    /// Maps parsed calls to stable symbol ids currently stored in the index.
    fn remap_calls(
        &self,
        parse_cache: &HashMap<String, FileParseResult>,
        db_symbols: &[repoctx_schema::artifacts::SymbolRecord],
    ) -> Vec<ParsedCall> {
        let mut remapped = Vec::new();

        for parsed in parse_cache.values() {
            let parse_id_to_name: HashMap<&str, &str> = parsed
                .symbols
                .iter()
                .map(|s| (s.id.as_str(), s.name.as_str()))
                .collect();

            for call in &parsed.calls {
                let Some(caller_name) = parse_id_to_name.get(call.caller_symbol_id.as_str()) else {
                    continue;
                };
                let Some(db_caller) = db_symbols
                    .iter()
                    .find(|s| s.file_path == parsed.path && s.name == *caller_name)
                else {
                    continue;
                };
                remapped.push(ParsedCall {
                    caller_symbol_id: db_caller.id.clone(),
                    callee_name: call.callee_name.clone(),
                });
            }
        }

        remapped
    }

    /// Indexes `main` functions as program entrypoints.
    fn index_entrypoints(
        &self,
        store: &IndexStore,
        symbols: &[repoctx_schema::artifacts::SymbolRecord],
    ) -> Result<usize, CoreError> {
        let mut count = 0usize;
        for symbol in symbols {
            if symbol.name == "main" {
                store.insert_entrypoint(&EntrypointRecord {
                    id: stable_entrypoint_id(&symbol.id, "main"),
                    symbol_id: symbol.id.clone(),
                    kind: EntrypointKind::Main,
                    label: Some(symbol.file_path.clone()),
                })?;
                count += 1;
            }
        }
        Ok(count)
    }
}

fn collect_imports(
    parse_cache: &HashMap<String, FileParseResult>,
) -> Vec<crate::parse::ParsedImport> {
    parse_cache
        .values()
        .flat_map(|parsed| parsed.imports.iter().cloned())
        .collect()
}
