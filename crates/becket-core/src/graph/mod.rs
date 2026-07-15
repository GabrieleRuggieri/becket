//! Multi-level symbol index for precise call and import resolution.
//!
//! Resolution ladder (strongest evidence first):
//! 1. same file (`FileScoped`)
//! 2. through the importing file's import table (`ImportResolved`)
//! 3. same directory (`DirScoped`)
//! 4. unique name across the repo (`GlobalUnique`)
//! 5. ambiguous name → capped candidate edges (`Candidate`) instead of a
//!    silent false negative.
//!
//! Every edge carries a `resolution` tier and the derived `confidence`, so
//! ranking and prompts can weigh syntactic guesses below verified links.

mod imports;

pub use imports::{FileImportTable, ImportResolver};

use std::collections::{HashMap, HashSet};
use std::path::Path;

use becket_schema::artifacts::{DependencyEdgeRecord, SymbolRecord};
use becket_schema::edge::{EdgeResolution, EdgeType};
use becket_schema::symbol::Visibility;

use crate::ids::stable_edge_id;
use crate::parse::{ParsedCall, ParsedImport, ParsedInheritance};

/// Maximum candidate edges emitted for one ambiguous call site.
pub const MAX_CANDIDATE_EDGES: usize = 3;
/// Ambiguous names with more repo-wide matches than this are dropped entirely
/// (utility names like `get`/`init` would otherwise flood the graph).
pub const MAX_AMBIGUOUS_MATCHES: usize = 8;

/// Outcome of resolving one reference.
pub enum ResolvedRef<'a> {
    /// Unambiguous target with the tier that produced it.
    One(&'a SymbolRecord, EdgeResolution),
    /// Ambiguous: capped, deterministic candidate set.
    Candidates(Vec<&'a SymbolRecord>),
    /// No plausible target in the repository.
    Unresolved,
}

/// In-memory index for O(1) symbol lookup during graph resolution.
pub struct SymbolIndex<'a> {
    by_id: HashMap<&'a str, &'a SymbolRecord>,
    by_file_and_name: HashMap<(&'a str, &'a str), Vec<&'a SymbolRecord>>,
    by_dir_and_name: HashMap<(String, &'a str), Vec<&'a SymbolRecord>>,
    by_name: HashMap<&'a str, Vec<&'a SymbolRecord>>,
}

impl<'a> SymbolIndex<'a> {
    /// Builds an index from the symbol catalog.
    pub fn new(symbols: &'a [SymbolRecord]) -> Self {
        let mut by_id = HashMap::with_capacity(symbols.len());
        let mut by_file_and_name: HashMap<(&'a str, &'a str), Vec<&'a SymbolRecord>> =
            HashMap::new();
        let mut by_dir_and_name: HashMap<(String, &'a str), Vec<&'a SymbolRecord>> = HashMap::new();
        let mut by_name: HashMap<&'a str, Vec<&'a SymbolRecord>> = HashMap::new();

        for symbol in symbols {
            by_id.insert(symbol.id.as_str(), symbol);
            by_file_and_name
                .entry((symbol.file_path.as_str(), symbol.name.as_str()))
                .or_default()
                .push(symbol);

            if let Some(dir) = parent_dir(&symbol.file_path) {
                by_dir_and_name
                    .entry((dir, symbol.name.as_str()))
                    .or_default()
                    .push(symbol);
            }

            by_name
                .entry(symbol.name.as_str())
                .or_default()
                .push(symbol);
        }

        Self {
            by_id,
            by_file_and_name,
            by_dir_and_name,
            by_name,
        }
    }

    /// Looks up a symbol by stable id.
    pub fn by_id(&self, id: &str) -> Option<&'a SymbolRecord> {
        self.by_id.get(id).copied()
    }

    /// Resolves a callee using the resolution ladder (see module docs).
    pub fn resolve_ref(
        &self,
        caller_file: &str,
        callee_name: &str,
        import_table: Option<&FileImportTable>,
    ) -> ResolvedRef<'a> {
        // 1. Same file.
        if let Some(matches) = self.by_file_and_name.get(&(caller_file, callee_name)) {
            if let Some(first) = pick_deterministic(matches) {
                return ResolvedRef::One(first, EdgeResolution::FileScoped);
            }
        }

        // 2. Through the file's imports.
        if let Some(table) = import_table {
            if let Some(target_files) = table.targets_for(callee_name) {
                let mut found: Vec<&'a SymbolRecord> = Vec::new();
                for file in target_files {
                    if let Some(matches) = self.by_file_and_name.get(&(file.as_str(), callee_name))
                    {
                        found.extend(matches.iter().copied());
                    }
                }
                if let Some(first) = pick_deterministic(&found) {
                    return ResolvedRef::One(first, EdgeResolution::ImportResolved);
                }
            }
        }

        // 3. Same directory (unique or unique-public).
        if let Some(dir) = parent_dir(caller_file) {
            if let Some(candidates) = self.by_dir_and_name.get(&(dir, callee_name)) {
                if let Some(resolved) = disambiguate(candidates) {
                    return ResolvedRef::One(resolved, EdgeResolution::DirScoped);
                }
            }
        }

        // 4./5. Global.
        match self.by_name.get(callee_name) {
            Some(candidates) if candidates.len() == 1 => {
                ResolvedRef::One(candidates[0], EdgeResolution::GlobalUnique)
            }
            Some(candidates) => {
                if let Some(resolved) = disambiguate(candidates) {
                    return ResolvedRef::One(resolved, EdgeResolution::GlobalUnique);
                }
                if candidates.len() > MAX_AMBIGUOUS_MATCHES {
                    return ResolvedRef::Unresolved;
                }
                let mut sorted: Vec<&'a SymbolRecord> = candidates.clone();
                sorted.sort_by(|a, b| {
                    a.file_path
                        .cmp(&b.file_path)
                        .then_with(|| a.start_line.cmp(&b.start_line))
                });
                sorted.truncate(MAX_CANDIDATE_EDGES);
                ResolvedRef::Candidates(sorted)
            }
            None => ResolvedRef::Unresolved,
        }
    }
}

/// Inputs for full graph resolution.
pub struct ResolveInput<'a> {
    /// Symbol catalog (stable ids).
    pub symbols: &'a [SymbolRecord],
    /// Unresolved call references.
    pub calls: &'a [ParsedCall],
    /// Unresolved import declarations.
    pub imports: &'a [ParsedImport],
    /// Unresolved inheritance references.
    pub inheritance: &'a [ParsedInheritance],
    /// Every repository-relative source path (for module resolution).
    pub known_files: &'a HashSet<String>,
}

/// Resolution result: edges plus counters for the build report.
pub struct ResolveOutput {
    /// Deterministic, deduplicated dependency edges.
    pub edges: Vec<DependencyEdgeRecord>,
    /// Call sites with no plausible target in the repository.
    pub unresolved_calls: usize,
}

/// Builds dependency edges from calls, imports, and inheritance.
pub struct GraphResolver;

impl GraphResolver {
    /// Resolves all references to edges with per-edge resolution tiers.
    pub fn resolve_all(input: ResolveInput<'_>) -> ResolveOutput {
        let index = SymbolIndex::new(input.symbols);
        let import_resolver = ImportResolver::build(input.imports, input.known_files);
        let mut edges = Vec::new();
        let mut seen = HashSet::new();
        let mut unresolved_calls = 0usize;

        for call in input.calls {
            let Some(caller) = index.by_id(&call.caller_symbol_id) else {
                continue;
            };
            let table = import_resolver.table_for(&caller.file_path);
            match index.resolve_ref(&caller.file_path, &call.callee_name, table) {
                ResolvedRef::One(callee, resolution) => {
                    if caller.id != callee.id {
                        push_edge(
                            &mut edges,
                            &mut seen,
                            &caller.id,
                            &callee.id,
                            EdgeType::Calls,
                            resolution,
                        );
                    }
                }
                ResolvedRef::Candidates(candidates) => {
                    for callee in candidates {
                        if caller.id != callee.id {
                            push_edge(
                                &mut edges,
                                &mut seen,
                                &caller.id,
                                &callee.id,
                                EdgeType::Calls,
                                EdgeResolution::Candidate,
                            );
                        }
                    }
                }
                ResolvedRef::Unresolved => unresolved_calls += 1,
            }
        }

        // File-level imports: attach to the file's first declared symbol
        // (documented approximation until file nodes exist) and resolve the
        // target through the import table first.
        let mut first_symbol_by_file: HashMap<&str, &SymbolRecord> = HashMap::new();
        for symbol in input.symbols {
            first_symbol_by_file
                .entry(symbol.file_path.as_str())
                .and_modify(|existing| {
                    if symbol.start_line < existing.start_line {
                        *existing = symbol;
                    }
                })
                .or_insert(symbol);
        }

        for import in input.imports {
            let Some(importer_symbol) = first_symbol_by_file.get(import.file_path.as_str()) else {
                continue;
            };
            let table = import_resolver.table_for(&import.file_path);
            let local_name = import.alias.as_deref().unwrap_or(&import.imported_name);
            match index.resolve_ref(&import.file_path, local_name, table) {
                ResolvedRef::One(target, resolution) => {
                    if importer_symbol.id != target.id {
                        push_edge(
                            &mut edges,
                            &mut seen,
                            &importer_symbol.id,
                            &target.id,
                            EdgeType::Imports,
                            resolution,
                        );
                    }
                }
                // Ambiguous imports are dropped: import edges are a weak
                // signal already, candidates would only add noise.
                ResolvedRef::Candidates(_) | ResolvedRef::Unresolved => {}
            }
        }

        for edge in input.inheritance {
            let Some(child) = index.by_id(&edge.child_symbol_id) else {
                continue;
            };
            let table = import_resolver.table_for(&child.file_path);
            match index.resolve_ref(&child.file_path, &edge.parent_name, table) {
                ResolvedRef::One(parent, resolution) => {
                    if child.id != parent.id {
                        push_edge(
                            &mut edges,
                            &mut seen,
                            &child.id,
                            &parent.id,
                            edge.edge_type,
                            resolution,
                        );
                    }
                }
                ResolvedRef::Candidates(candidates) => {
                    for parent in candidates {
                        if child.id != parent.id {
                            push_edge(
                                &mut edges,
                                &mut seen,
                                &child.id,
                                &parent.id,
                                edge.edge_type,
                                EdgeResolution::Candidate,
                            );
                        }
                    }
                }
                ResolvedRef::Unresolved => {}
            }
        }

        edges.sort_by(|a, b| {
            a.src_symbol_id
                .cmp(&b.src_symbol_id)
                .then_with(|| a.dst_symbol_id.cmp(&b.dst_symbol_id))
                .then_with(|| edge_type_as_str(a.edge_type).cmp(edge_type_as_str(b.edge_type)))
        });

        ResolveOutput {
            edges,
            unresolved_calls,
        }
    }

    /// Backward-compatible resolution without an explicit file set.
    pub fn resolve(
        symbols: &[SymbolRecord],
        calls: &[ParsedCall],
        imports: &[ParsedImport],
        inheritance: &[ParsedInheritance],
    ) -> Vec<DependencyEdgeRecord> {
        let known_files: HashSet<String> = symbols.iter().map(|s| s.file_path.clone()).collect();
        Self::resolve_all(ResolveInput {
            symbols,
            calls,
            imports,
            inheritance,
            known_files: &known_files,
        })
        .edges
    }

    /// Backward-compatible call-only resolution.
    pub fn resolve_calls(
        symbols: &[SymbolRecord],
        calls: &[ParsedCall],
    ) -> Vec<DependencyEdgeRecord> {
        Self::resolve(symbols, calls, &[], &[])
    }
}

fn push_edge(
    edges: &mut Vec<DependencyEdgeRecord>,
    seen: &mut HashSet<String>,
    src: &str,
    dst: &str,
    edge_type: EdgeType,
    resolution: EdgeResolution,
) {
    let type_str = edge_type_as_str(edge_type);
    let id = stable_edge_id(src, dst, type_str);
    if !seen.insert(id.clone()) {
        return;
    }
    edges.push(DependencyEdgeRecord {
        id,
        src_symbol_id: src.to_string(),
        dst_symbol_id: dst.to_string(),
        edge_type,
        boundary: None,
        confidence: resolution.confidence(),
        resolution,
    });
}

fn edge_type_as_str(edge_type: EdgeType) -> &'static str {
    match edge_type {
        EdgeType::Calls => "calls",
        EdgeType::Imports => "imports",
        EdgeType::Extends => "extends",
        EdgeType::Implements => "implements",
        EdgeType::References => "references",
        EdgeType::Reads => "reads",
        EdgeType::Writes => "writes",
        EdgeType::Http => "http",
        EdgeType::Grpc => "grpc",
        EdgeType::Queue => "queue",
    }
}

fn parent_dir(file_path: &str) -> Option<String> {
    Path::new(file_path)
        .parent()
        .and_then(|p| p.to_str())
        .filter(|p| !p.is_empty())
        .map(str::to_string)
}

/// Picks the earliest declaration when several same-name symbols share a scope.
fn pick_deterministic<'a>(candidates: &[&'a SymbolRecord]) -> Option<&'a SymbolRecord> {
    candidates
        .iter()
        .min_by_key(|s| (s.file_path.as_str(), s.start_line))
        .copied()
}

fn disambiguate<'a>(candidates: &[&'a SymbolRecord]) -> Option<&'a SymbolRecord> {
    if candidates.len() == 1 {
        return Some(candidates[0]);
    }
    let public: Vec<_> = candidates
        .iter()
        .filter(|s| s.visibility == Visibility::Public)
        .copied()
        .collect();
    if public.len() == 1 {
        return Some(public[0]);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use becket_schema::symbol::{SymbolKind, Visibility};

    fn sym(id: &str, name: &str, file: &str) -> SymbolRecord {
        SymbolRecord {
            id: id.into(),
            kind: SymbolKind::Function,
            name: name.into(),
            fqn: format!("{file}::{name}"),
            file_path: file.into(),
            start_line: 1,
            end_line: 1,
            visibility: Visibility::Public,
            module_id: None,
        }
    }

    fn class_sym(id: &str, name: &str, file: &str) -> SymbolRecord {
        SymbolRecord {
            kind: SymbolKind::Class,
            ..sym(id, name, file)
        }
    }

    #[test]
    fn resolves_extends_and_implements() {
        use crate::parse::ParsedInheritance;

        let symbols = vec![
            class_sym("shape", "Shape", "src/shapes.ts"),
            class_sym("circle", "Circle", "src/shapes.ts"),
            SymbolRecord {
                kind: SymbolKind::Type,
                ..sym("pet", "Pet", "src/Animals.java")
            },
            class_sym("animal", "Animal", "src/Animals.java"),
            class_sym("dog", "Dog", "src/Animals.java"),
        ];
        let inheritance = vec![
            ParsedInheritance {
                child_symbol_id: "circle".into(),
                parent_name: "Shape".into(),
                edge_type: EdgeType::Extends,
            },
            ParsedInheritance {
                child_symbol_id: "dog".into(),
                parent_name: "Animal".into(),
                edge_type: EdgeType::Extends,
            },
            ParsedInheritance {
                child_symbol_id: "dog".into(),
                parent_name: "Pet".into(),
                edge_type: EdgeType::Implements,
            },
        ];
        let edges = GraphResolver::resolve(&symbols, &[], &[], &inheritance);
        assert_eq!(edges.len(), 3);
        assert!(edges.iter().any(|e| e.edge_type == EdgeType::Extends));
        assert!(edges.iter().any(|e| e.edge_type == EdgeType::Implements));
        assert!(edges
            .iter()
            .all(|e| e.resolution == EdgeResolution::FileScoped));
    }

    #[test]
    fn resolves_call_chain() {
        let symbols = vec![
            sym("a", "func_a", "src/g.rs"),
            sym("b", "func_b", "src/g.rs"),
            sym("c", "func_c", "src/g.rs"),
        ];
        let calls = vec![
            ParsedCall {
                caller_symbol_id: "a".into(),
                callee_name: "func_b".into(),
            },
            ParsedCall {
                caller_symbol_id: "b".into(),
                callee_name: "func_c".into(),
            },
        ];
        let edges = GraphResolver::resolve_calls(&symbols, &calls);
        assert_eq!(edges.len(), 2);
        assert!(edges
            .iter()
            .all(|e| e.confidence == EdgeResolution::FileScoped.confidence()));
    }

    #[test]
    fn prefers_same_file_over_global_duplicate() {
        let symbols = vec![
            sym("a", "helper", "src/a.rs"),
            sym("b", "helper", "src/b.rs"),
            sym("c", "caller", "src/a.rs"),
        ];
        let calls = vec![ParsedCall {
            caller_symbol_id: "c".into(),
            callee_name: "helper".into(),
        }];
        let edges = GraphResolver::resolve_calls(&symbols, &calls);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].dst_symbol_id, "a");
        assert_eq!(edges[0].resolution, EdgeResolution::FileScoped);
    }

    #[test]
    fn import_table_beats_directory_and_global_guesses() {
        let symbols = vec![
            sym("caller", "handler", "src/api/handler.py"),
            sym("right", "charge", "src/payment/gateway.py"),
            sym("wrong", "charge", "src/legacy/old_gateway.py"),
        ];
        let calls = vec![ParsedCall {
            caller_symbol_id: "caller".into(),
            callee_name: "charge".into(),
        }];
        let imports = vec![ParsedImport {
            file_path: "src/api/handler.py".into(),
            imported_name: "charge".into(),
            module_path: Some("src.payment.gateway".into()),
            alias: None,
        }];
        let known: HashSet<String> = symbols.iter().map(|s| s.file_path.clone()).collect();
        let output = GraphResolver::resolve_all(ResolveInput {
            symbols: &symbols,
            calls: &calls,
            imports: &imports,
            inheritance: &[],
            known_files: &known,
        });
        let call_edges: Vec<_> = output
            .edges
            .iter()
            .filter(|e| e.edge_type == EdgeType::Calls)
            .collect();
        assert_eq!(call_edges.len(), 1);
        assert_eq!(call_edges[0].dst_symbol_id, "right");
        assert_eq!(call_edges[0].resolution, EdgeResolution::ImportResolved);
    }

    #[test]
    fn ambiguous_global_emits_capped_candidates_not_silence() {
        let mut symbols = vec![sym("caller", "run", "src/main.rs")];
        for i in 0..3 {
            symbols.push(SymbolRecord {
                visibility: Visibility::Private,
                ..sym(&format!("t{i}"), "process", &format!("src/mod{i}/lib.rs"))
            });
        }
        let calls = vec![ParsedCall {
            caller_symbol_id: "caller".into(),
            callee_name: "process".into(),
        }];
        let edges = GraphResolver::resolve_calls(&symbols, &calls);
        assert_eq!(edges.len(), MAX_CANDIDATE_EDGES.min(3));
        assert!(edges
            .iter()
            .all(|e| e.resolution == EdgeResolution::Candidate));
        assert!(edges
            .iter()
            .all(|e| (e.confidence - EdgeResolution::Candidate.confidence()).abs() < f32::EPSILON));
    }

    #[test]
    fn very_common_names_are_dropped() {
        let mut symbols = vec![sym("caller", "run", "src/main.rs")];
        for i in 0..(MAX_AMBIGUOUS_MATCHES + 2) {
            symbols.push(SymbolRecord {
                visibility: Visibility::Private,
                ..sym(&format!("g{i}"), "get", &format!("src/m{i}/lib.rs"))
            });
        }
        let calls = vec![ParsedCall {
            caller_symbol_id: "caller".into(),
            callee_name: "get".into(),
        }];
        let known: HashSet<String> = symbols.iter().map(|s| s.file_path.clone()).collect();
        let output = GraphResolver::resolve_all(ResolveInput {
            symbols: &symbols,
            calls: &calls,
            imports: &[],
            inheritance: &[],
            known_files: &known,
        });
        assert!(output.edges.is_empty());
        assert_eq!(output.unresolved_calls, 1);
    }

    #[test]
    fn edge_ids_are_deterministic() {
        let symbols = vec![sym("a", "f", "src/a.rs"), sym("b", "g", "src/a.rs")];
        let calls = vec![ParsedCall {
            caller_symbol_id: "a".into(),
            callee_name: "g".into(),
        }];
        let e1 = GraphResolver::resolve_calls(&symbols, &calls);
        let e2 = GraphResolver::resolve_calls(&symbols, &calls);
        assert_eq!(e1[0].id, e2[0].id);
    }
}
