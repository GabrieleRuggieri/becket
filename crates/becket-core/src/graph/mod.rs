//! Multi-level symbol index for precise call and import resolution.

use std::collections::HashMap;
use std::path::Path;

use becket_schema::artifacts::{DependencyEdgeRecord, SymbolRecord};
use becket_schema::edge::EdgeType;
use becket_schema::symbol::Visibility;

use crate::ids::stable_edge_id;
use crate::parse::{ParsedCall, ParsedImport, ParsedInheritance};

/// In-memory index for O(1) symbol lookup during graph resolution.
pub struct SymbolIndex<'a> {
    by_id: HashMap<&'a str, &'a SymbolRecord>,
    by_file_and_name: HashMap<(&'a str, &'a str), &'a SymbolRecord>,
    by_dir_and_name: HashMap<(String, &'a str), Vec<&'a SymbolRecord>>,
    by_name: HashMap<&'a str, Vec<&'a SymbolRecord>>,
}

impl<'a> SymbolIndex<'a> {
    /// Builds an index from the symbol catalog.
    pub fn new(symbols: &'a [SymbolRecord]) -> Self {
        let mut by_id = HashMap::with_capacity(symbols.len());
        let mut by_file_and_name = HashMap::new();
        let mut by_dir_and_name: HashMap<(String, &'a str), Vec<&'a SymbolRecord>> = HashMap::new();
        let mut by_name: HashMap<&'a str, Vec<&'a SymbolRecord>> = HashMap::new();

        for symbol in symbols {
            by_id.insert(symbol.id.as_str(), symbol);
            by_file_and_name.insert((symbol.file_path.as_str(), symbol.name.as_str()), symbol);

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

    /// Resolves a callee from a call site using scoped lookup (file → directory → unique global).
    pub fn resolve_call(
        &self,
        caller: &SymbolRecord,
        callee_name: &str,
    ) -> Option<&'a SymbolRecord> {
        if let Some(symbol) = self
            .by_file_and_name
            .get(&(caller.file_path.as_str(), callee_name))
        {
            return Some(symbol);
        }

        if let Some(dir) = parent_dir(&caller.file_path) {
            if let Some(candidates) = self.by_dir_and_name.get(&(dir, callee_name)) {
                if let Some(resolved) = disambiguate(candidates) {
                    return Some(resolved);
                }
            }
        }

        self.by_name
            .get(callee_name)
            .and_then(|candidates| disambiguate(candidates))
    }

    /// Resolves an imported name to a symbol (same rules as calls).
    pub fn resolve_import(
        &self,
        importer: &SymbolRecord,
        imported_name: &str,
    ) -> Option<&'a SymbolRecord> {
        self.resolve_call(importer, imported_name)
    }
}

/// Builds dependency edges from calls and imports with deterministic ids.
pub struct GraphResolver;

impl GraphResolver {
    /// Resolves call, import, and inheritance references to edges.
    pub fn resolve(
        symbols: &[SymbolRecord],
        calls: &[ParsedCall],
        imports: &[ParsedImport],
        inheritance: &[ParsedInheritance],
    ) -> Vec<DependencyEdgeRecord> {
        let index = SymbolIndex::new(symbols);
        let mut edges = Vec::new();
        let mut seen = HashMap::new();

        for call in calls {
            let Some(caller) = index.by_id.get(call.caller_symbol_id.as_str()) else {
                continue;
            };
            let Some(callee) = index.resolve_call(caller, &call.callee_name) else {
                continue;
            };
            if caller.id == callee.id {
                continue;
            }
            push_edge(
                &mut edges,
                &mut seen,
                &caller.id,
                &callee.id,
                EdgeType::Calls,
                1.0,
            );
        }

        for import in imports {
            let Some(importer_symbol) = symbols
                .iter()
                .filter(|s| s.file_path == import.file_path)
                .min_by_key(|s| s.start_line)
            else {
                continue;
            };
            let Some(target) = index.resolve_import(importer_symbol, &import.imported_name) else {
                continue;
            };
            push_edge(
                &mut edges,
                &mut seen,
                &importer_symbol.id,
                &target.id,
                EdgeType::Imports,
                1.0,
            );
        }

        for edge in inheritance {
            let Some(child) = index.by_id.get(edge.child_symbol_id.as_str()) else {
                continue;
            };
            let Some(parent) = index.resolve_call(child, &edge.parent_name) else {
                continue;
            };
            if child.id == parent.id {
                continue;
            }
            push_edge(
                &mut edges,
                &mut seen,
                &child.id,
                &parent.id,
                edge.edge_type,
                1.0,
            );
        }

        edges.sort_by(|a, b| {
            a.src_symbol_id
                .cmp(&b.src_symbol_id)
                .then_with(|| a.dst_symbol_id.cmp(&b.dst_symbol_id))
                .then_with(|| format!("{:?}", a.edge_type).cmp(&format!("{:?}", b.edge_type)))
        });
        edges
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
    seen: &mut HashMap<String, ()>,
    src: &str,
    dst: &str,
    edge_type: EdgeType,
    confidence: f32,
) {
    let type_str = edge_type_as_str(edge_type);
    let id = stable_edge_id(src, dst, type_str);
    if seen.insert(id.clone(), ()).is_some() {
        return;
    }
    edges.push(DependencyEdgeRecord {
        id,
        src_symbol_id: src.to_string(),
        dst_symbol_id: dst.to_string(),
        edge_type,
        boundary: None,
        confidence,
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
