//! Graph-grounded wiki page compiler (template slots + claim blocks).

use std::collections::{HashMap, HashSet};

use repoctx_schema::artifacts::{FlowRecord, ModuleRecord, SymbolRecord};
use repoctx_schema::wiki::{WikiPageKind, WikiPageMeta, WikiPageSource};
use repoctx_store::{IndexStore, RepoCtxPaths};

use crate::error::CoreError;
use crate::wiki::fingerprint::subgraph_fingerprint;
use crate::wiki::store::WikiStore;
use crate::wiki::util::{
    format_call_edge, merge_preserved_prose, prose_slot, sym_name, symbol_name_map,
};

/// Compiles wiki pages from the deterministic graph.
pub struct WikiCompiler {
    paths: RepoCtxPaths,
}

impl WikiCompiler {
    /// Creates a compiler for the repository at `paths`.
    pub fn new(paths: RepoCtxPaths) -> Self {
        Self { paths }
    }

    /// Compiles all wiki pages and `index.md`. Returns number of pages written.
    pub fn compile_all(&self, store: &IndexStore) -> Result<usize, CoreError> {
        self.write_pages(store, &[], false)
    }

    /// Recompiles stale pages (or all when `page_ids` is empty), preserving enriched prose.
    pub fn sync_pages(&self, store: &IndexStore, page_ids: &[String]) -> Result<usize, CoreError> {
        self.write_pages(store, page_ids, true)
    }

    fn write_pages(
        &self,
        store: &IndexStore,
        only_ids: &[String],
        preserve_prose: bool,
    ) -> Result<usize, CoreError> {
        let wiki_store = WikiStore::new(&self.paths);
        wiki_store.ensure_dir()?;

        let mut pages = self.build_pages(store)?;
        wire_see_also(&mut pages);

        let filter: HashSet<&str> = only_ids.iter().map(String::as_str).collect();
        let write_all = only_ids.is_empty();

        let index_body = render_index(&pages);
        let index_meta = WikiPageMeta {
            id: "wiki_index".into(),
            kind: WikiPageKind::Overview,
            symbol_ids: Vec::new(),
            source: WikiPageSource::Deterministic,
            graph_fingerprint: "index".into(),
            see_also: pages.iter().map(|(m, _)| m.id.clone()).collect(),
            title: "Repo Wiki Index".into(),
        };
        wiki_store.write_index(&index_meta, &index_body)?;

        let mut count = 1usize;
        for (meta, body) in &pages {
            if !write_all && !filter.contains(meta.id.as_str()) {
                continue;
            }
            let final_body = if preserve_prose {
                if let Ok(Some(existing)) = wiki_store.load_page(&meta.id) {
                    merge_preserved_prose(&existing.body, body)
                } else {
                    body.clone()
                }
            } else {
                body.clone()
            };
            wiki_store.write_page(meta, &final_body)?;
            count += 1;
        }

        Ok(count)
    }

    fn build_pages(&self, store: &IndexStore) -> Result<Vec<(WikiPageMeta, String)>, CoreError> {
        let symbols = store.load_symbols()?;
        let (_, _, flows_art, entrypoints_art, architecture) = store.export_artifacts()?;
        let flows = &flows_art.flows;
        let entrypoints = &entrypoints_art.entrypoints;
        let call_edges = store.load_call_edges()?;
        let names = symbol_name_map(&symbols);

        let entrypoint_symbols: HashSet<&str> =
            entrypoints.iter().map(|e| e.symbol_id.as_str()).collect();

        let mut pages: Vec<(WikiPageMeta, String)> = Vec::new();

        for flow in flows {
            let symbol_ids: Vec<String> = flow.steps.iter().map(|s| s.symbol_id.clone()).collect();
            let meta = WikiPageMeta {
                id: format!("wiki_flow_{}", slugify(&flow.name)),
                kind: WikiPageKind::Flow,
                symbol_ids: symbol_ids.clone(),
                source: WikiPageSource::Deterministic,
                graph_fingerprint: subgraph_fingerprint(store, &symbol_ids)?,
                see_also: Vec::new(),
                title: format!("Flow: {}", flow.name),
            };
            let body = render_flow_body(flow, &names, &call_edges);
            pages.push((meta, body));
        }

        for module in &architecture.modules {
            let meta = WikiPageMeta {
                id: format!("wiki_module_{}", slugify(&module.id)),
                kind: WikiPageKind::Module,
                symbol_ids: module.symbol_ids.clone(),
                source: WikiPageSource::Deterministic,
                graph_fingerprint: subgraph_fingerprint(store, &module.symbol_ids)?,
                see_also: Vec::new(),
                title: format!("Module: {}", module.name),
            };
            let body = render_module_body(module, &symbols, &names, store)?;
            pages.push((meta, body));
        }

        for symbol in &symbols {
            if !entrypoint_symbols.contains(symbol.id.as_str()) {
                continue;
            }
            let symbol_ids = vec![symbol.id.clone()];
            let meta = WikiPageMeta {
                id: format!("wiki_service_{}", slugify(&symbol.name)),
                kind: WikiPageKind::Service,
                symbol_ids: symbol_ids.clone(),
                source: WikiPageSource::Deterministic,
                graph_fingerprint: subgraph_fingerprint(store, &symbol_ids)?,
                see_also: Vec::new(),
                title: format!("Service: {}", symbol.name),
            };
            let body = render_service_body(symbol, &symbols, &names, store, &call_edges)?;
            pages.push((meta, body));
        }

        Ok(pages)
    }
}

/// Links flow ↔ service ↔ module pages via `see_also`.
fn wire_see_also(pages: &mut [(WikiPageMeta, String)]) {
    let sym_to_pages: HashMap<String, Vec<String>> = {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for (meta, _) in pages.iter() {
            for sym in &meta.symbol_ids {
                map.entry(sym.clone()).or_default().push(meta.id.clone());
            }
        }
        map
    };

    for (meta, _) in pages.iter_mut() {
        let mut links: HashSet<String> = HashSet::new();
        for sym in &meta.symbol_ids {
            if let Some(related) = sym_to_pages.get(sym) {
                for id in related {
                    if id != &meta.id {
                        links.insert(id.clone());
                    }
                }
            }
        }
        let mut see_also: Vec<String> = links.into_iter().collect();
        see_also.sort();
        meta.see_also = see_also;
    }
}

fn slugify(input: &str) -> String {
    input
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .to_lowercase()
}

fn render_index(pages: &[(WikiPageMeta, String)]) -> String {
    let mut md = String::from("# Repo Wiki\n\n");
    let mut by_kind: HashMap<WikiPageKind, Vec<&WikiPageMeta>> = HashMap::new();
    for (meta, _) in pages {
        by_kind.entry(meta.kind).or_default().push(meta);
    }
    for kind in [
        WikiPageKind::Flow,
        WikiPageKind::Service,
        WikiPageKind::Module,
    ] {
        let Some(items) = by_kind.get(&kind) else {
            continue;
        };
        md.push_str(&format!("## {kind:?}\n\n"));
        for meta in items {
            let stem = meta.id.strip_prefix("wiki_").unwrap_or(&meta.id);
            md.push_str(&format!("- [{title}]({stem}.md)\n", title = meta.title));
        }
        md.push('\n');
    }
    md
}

fn render_flow_body(
    flow: &FlowRecord,
    names: &HashMap<&str, &str>,
    call_edges: &[(String, String)],
) -> String {
    let mut md = format!("# {}\n\n## Execution path\n\n", flow.name);
    for step in &flow.steps {
        let name = sym_name(&step.symbol_id, names);
        md.push_str(&format!(
            "{}. **{}** (`{}`)\n",
            step.order + 1,
            name,
            step.symbol_id
        ));
        if let Some(ext) = &step.external_system {
            md.push_str(&format!("   - external: {ext}\n"));
        }
    }
    md.push_str("\n## Call edges (flow subgraph)\n\n");
    let flow_ids: HashSet<&str> = flow.steps.iter().map(|s| s.symbol_id.as_str()).collect();
    for (src, dst) in call_edges {
        if flow_ids.contains(src.as_str()) && flow_ids.contains(dst.as_str()) {
            md.push_str(&format_call_edge(src, dst, names));
        }
    }
    md.push_str(&prose_slot());
    md
}

fn render_module_body(
    module: &ModuleRecord,
    symbols: &[SymbolRecord],
    names: &HashMap<&str, &str>,
    store: &IndexStore,
) -> Result<String, CoreError> {
    let mut md = format!("# {}\n\n## Symbols\n\n", module.name);
    for sym_id in &module.symbol_ids {
        let Some(sym) = symbols.iter().find(|s| &s.id == sym_id) else {
            continue;
        };
        md.push_str(&format!(
            "- **{}** — `{}:{}-{}`\n",
            sym.name, sym.file_path, sym.start_line, sym.end_line
        ));
    }
    md.push_str("\n## Impact summary\n\n");
    for sym_id in module.symbol_ids.iter().take(5) {
        let downstream = store.downstream_symbols(sym_id, 1)?;
        if !downstream.is_empty() {
            md.push_str(&format!(
                "- **{}** affects {} symbols (depth 1)\n",
                sym_name(sym_id, names),
                downstream.len()
            ));
        }
    }
    md.push_str(&prose_slot());
    Ok(md)
}

fn render_service_body(
    symbol: &SymbolRecord,
    symbols: &[SymbolRecord],
    names: &HashMap<&str, &str>,
    store: &IndexStore,
    call_edges: &[(String, String)],
) -> Result<String, CoreError> {
    let id_to_sym: HashMap<&str, &SymbolRecord> =
        symbols.iter().map(|s| (s.id.as_str(), s)).collect();

    let mut md = format!(
        "# {}\n\n## Structure\n\n**Location:** `{}:{}-{}`\n\n",
        symbol.name, symbol.file_path, symbol.start_line, symbol.end_line
    );

    md.push_str("### Callers\n\n");
    let callers: Vec<_> = call_edges
        .iter()
        .filter(|(_, dst)| dst == &symbol.id)
        .collect();
    if callers.is_empty() {
        md.push_str("_None detected._\n\n");
    } else {
        for (src, dst) in callers {
            md.push_str(&format_call_edge(src, dst, names));
        }
        md.push('\n');
    }

    md.push_str("### Callees\n\n");
    let callees: Vec<_> = call_edges
        .iter()
        .filter(|(src, _)| src == &symbol.id)
        .collect();
    if callees.is_empty() {
        md.push_str("_None detected._\n\n");
    } else {
        for (src, dst) in callees {
            md.push_str(&format_call_edge(src, dst, names));
        }
        md.push('\n');
    }

    let affected = store.downstream_symbols(&symbol.id, 2)?;
    md.push_str("## Impact\n\n");
    if affected.is_empty() {
        md.push_str("_No downstream symbols within depth 2._\n\n");
    } else {
        for id in affected.iter().take(15) {
            if let Some(sym) = id_to_sym.get(id.as_str()) {
                md.push_str(&format!(
                    "- **{}** — `{}:{}-{}`\n",
                    sym.name, sym.file_path, sym.start_line, sym.end_line
                ));
            } else {
                md.push_str(&format!("- **{}**\n", sym_name(id, names)));
            }
        }
        if affected.len() > 15 {
            md.push_str(&format!("\n_…and {} more_\n", affected.len() - 15));
        }
        md.push('\n');
    }

    md.push_str(&prose_slot());
    Ok(md)
}
