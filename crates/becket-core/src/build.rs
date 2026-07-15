//! `becket build` orchestration: walk, hash, extract, persist, emit artifacts.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use becket_schema::artifacts::EntrypointRecord;
use becket_schema::edge::EdgeType;
use becket_schema::symbol::EntrypointKind;
use becket_store::{ArtifactWriter, BecketPaths, IndexStore};
use tracing::{info, warn};

use crate::domain::apply_domain_overrides;
use crate::embed::index_symbol_embeddings;
use crate::error::CoreError;
use crate::flow::{CallEdge, FlowReconstructor};
use crate::graph::{GraphResolver, ResolveInput};
use crate::history::mine_co_change;
use crate::ids::{stable_entrypoint_id, stable_file_id};
use crate::parse::{FileParseResult, TreeSitterParser};
use crate::walker::{FileWalker, SourceFile};
use crate::wiki::{WikiCompiler, WikiLinter};

/// Meta key storing the embedding space id used at build time.
pub const META_EMBEDDER: &str = "embedder";
/// Meta key storing the last build report as JSON (consumed by `becket report`).
pub const META_LAST_BUILD_REPORT: &str = "last_build_report";

/// Options controlling a build run.
#[derive(Debug, Clone)]
pub struct BuildOptions {
    /// When true, skip files whose content hash is unchanged.
    pub incremental: bool,
    /// When true, skip embedding generation.
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    /// Call sites with no plausible target in the repository.
    #[serde(default)]
    pub unresolved_calls: usize,
    /// Entrypoints detected.
    pub entrypoints_indexed: usize,
    /// Flows auto-discovered.
    pub flows_indexed: usize,
    /// Symbol embeddings indexed (when enabled).
    pub embeddings_indexed: usize,
    /// Wiki pages compiled under `.becket/wiki/`.
    pub wiki_pages_indexed: usize,
    /// Co-change file pairs mined from git history.
    #[serde(default)]
    pub co_change_pairs: usize,
    /// Path to `.becket/` output directory.
    pub output_dir: String,
}

/// End-to-end deterministic build pipeline.
pub struct BuildPipeline {
    paths: BecketPaths,
    options: BuildOptions,
}

impl BuildPipeline {
    /// Creates a pipeline for the repository at `root`.
    pub fn new(root: impl AsRef<Path>, options: BuildOptions) -> Self {
        Self {
            paths: BecketPaths::new(root),
            options,
        }
    }

    /// Runs the full build: walk → parse → index → emit JSON artifacts.
    ///
    /// All index writes happen inside one transaction: a failed build leaves
    /// the previous index intact instead of a half-written one.
    pub fn run(&self) -> Result<BuildReport, CoreError> {
        let store = IndexStore::open(&self.paths.index_db)?;
        store.begin()?;
        let result = self.run_inner(&store);
        match &result {
            Ok(_) => store.commit()?,
            Err(_) => store.rollback(),
        }
        result
    }

    fn run_inner(&self, store: &IndexStore) -> Result<BuildReport, CoreError> {
        let walker = FileWalker::new(&self.paths.root);
        let discovered = walker.discover()?;

        if !self.options.incremental {
            store.clear_all()?;
        }

        let live_paths: HashSet<String> =
            discovered.iter().map(|f| f.relative_path.clone()).collect();
        if self.options.incremental {
            let pruned = store.prune_missing_files(&live_paths)?;
            if pruned > 0 {
                info!(pruned, "removed deleted files from index");
            }
        }

        let mut files_parsed = 0usize;
        let mut files_skipped = 0usize;
        let mut symbols_indexed = 0usize;
        // BTreeMap: deterministic iteration order for downstream indexing.
        let mut parse_cache: BTreeMap<String, FileParseResult> = BTreeMap::new();

        for file in &discovered {
            // Incremental fast path: unchanged files reuse the cached parse
            // payload and are never re-read or re-parsed.
            if self.should_skip_file(store, file)? {
                if let Some(payload) = store.load_raw_refs(&file.relative_path)? {
                    match serde_json::from_str::<FileParseResult>(&payload) {
                        Ok(parsed) => {
                            parse_cache.insert(file.relative_path.clone(), parsed);
                            files_skipped += 1;
                            continue;
                        }
                        Err(error) => {
                            warn!(path = %file.relative_path, %error, "raw refs cache invalid; reparsing");
                        }
                    }
                }
            }

            let parsed = TreeSitterParser::parse_file(
                &file.relative_path,
                file.language,
                &file.absolute_path,
            )?;

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

            let payload = serde_json::to_string(&parsed)
                .map_err(|error| CoreError::Parse(error.to_string()))?;
            store.upsert_raw_refs(&file.relative_path, &payload)?;

            files_parsed += 1;
            info!(path = %file.relative_path, symbols = parsed.symbols.len(), "parsed file");
            parse_cache.insert(file.relative_path.clone(), parsed);
        }

        let all_symbols = store.load_symbols()?;

        // Parse-time symbol ids are stable (content-derived), so calls and
        // inheritance refs from both fresh and cached parses already point at
        // the ids stored in the index; no remapping pass is needed.
        let mut all_calls = Vec::new();
        let mut all_imports = Vec::new();
        let mut all_inheritance = Vec::new();
        for parsed in parse_cache.values() {
            all_calls.extend(parsed.calls.iter().cloned());
            all_imports.extend(parsed.imports.iter().cloned());
            all_inheritance.extend(parsed.inheritance.iter().cloned());
        }

        let resolve_output = GraphResolver::resolve_all(ResolveInput {
            symbols: &all_symbols,
            calls: &all_calls,
            imports: &all_imports,
            inheritance: &all_inheritance,
            known_files: &live_paths,
        });
        let edges = resolve_output.edges;
        store.clear_edges()?;
        for edge in &edges {
            store.insert_edge(edge)?;
        }

        store.clear_entrypoints()?;
        let entrypoints_indexed = self.index_entrypoints(store, &all_symbols, &parse_cache)?;

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

        let embeddings_indexed = if self.options.no_embeddings {
            0
        } else {
            store.clear_symbol_vectors()?;
            let count = index_symbol_embeddings(store, &all_symbols)?;
            store.set_meta(META_EMBEDDER, becket_embed::current_embedder_id())?;
            count
        };

        // Co-change mining (fails soft on non-git repositories).
        let co_change_rows = mine_co_change(&self.paths.root);
        store.replace_co_change(&co_change_rows)?;
        let co_change_pairs = co_change_rows.len();

        let writer = ArtifactWriter::new(self.paths.clone());
        let (symbols, dependencies, flows, entrypoints, architecture) = store.export_artifacts()?;

        writer.write_artifact("symbols", &symbols)?;
        writer.write_artifact("dependencies", &dependencies)?;
        writer.write_artifact("flows", &flows)?;
        writer.write_artifact("entrypoints", &entrypoints)?;
        writer.write_artifact("architecture", &architecture)?;

        let wiki_pages_indexed = WikiCompiler::new(self.paths.clone()).compile_all(store)?;
        let _lint = WikiLinter::new(self.paths.clone()).run(store)?;

        let report = BuildReport {
            files_discovered: discovered.len(),
            files_parsed,
            files_skipped,
            symbols_indexed,
            edges_indexed: edges.len(),
            unresolved_calls: resolve_output.unresolved_calls,
            entrypoints_indexed,
            flows_indexed,
            embeddings_indexed,
            wiki_pages_indexed,
            co_change_pairs,
            output_dir: writer.output_dir().display().to_string(),
        };

        if let Ok(json) = serde_json::to_string(&report) {
            store.set_meta(META_LAST_BUILD_REPORT, &json)?;
        }

        Ok(report)
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

    /// Indexes program and HTTP entrypoints from parse results.
    fn index_entrypoints(
        &self,
        store: &IndexStore,
        symbols: &[becket_schema::artifacts::SymbolRecord],
        parse_cache: &BTreeMap<String, FileParseResult>,
    ) -> Result<usize, CoreError> {
        let mut count = 0usize;
        let mut seen = std::collections::HashSet::new();

        for parsed in parse_cache.values() {
            for route in &parsed.http_routes {
                let Some(handler) = symbols
                    .iter()
                    .find(|s| s.file_path == route.file_path && s.name == route.handler_name)
                else {
                    continue;
                };
                let dedupe = format!("{}:{}", handler.id, route.label);
                if !seen.insert(dedupe) {
                    continue;
                }
                let kind_key = format!("http:{}", route.label);
                store.insert_entrypoint(&EntrypointRecord {
                    id: stable_entrypoint_id(&handler.id, &kind_key),
                    symbol_id: handler.id.clone(),
                    kind: EntrypointKind::Http,
                    label: Some(route.label.clone()),
                })?;
                count += 1;
            }
        }

        for symbol in symbols {
            if symbol.name != "main" {
                continue;
            }
            let dedupe = format!("{}:main", symbol.id);
            if !seen.insert(dedupe) {
                continue;
            }
            store.insert_entrypoint(&EntrypointRecord {
                id: stable_entrypoint_id(&symbol.id, "main"),
                symbol_id: symbol.id.clone(),
                kind: EntrypointKind::Main,
                label: Some(symbol.file_path.clone()),
            })?;
            count += 1;
        }
        Ok(count)
    }
}
