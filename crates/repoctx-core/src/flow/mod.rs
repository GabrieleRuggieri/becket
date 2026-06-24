//! Auto-discovers business flows from folder names and call-graph traversal.

use std::collections::{HashMap, HashSet, VecDeque};

use repoctx_schema::artifacts::{FlowRecord, FlowStepRecord, SymbolRecord};

use crate::ids::stable_flow_id;

/// Edge endpoints for call-graph walks.
#[derive(Debug, Clone)]
pub struct CallEdge {
    /// Source symbol id.
    pub src: String,
    /// Target symbol id.
    pub dst: String,
}

/// Reconstructs flows from path-derived domain names and call edges.
pub struct FlowReconstructor;

impl FlowReconstructor {
    /// Builds flow records from symbols and call edges.
    ///
    /// Domains are folder segments shared by at least two symbols (e.g. `payment/`).
    /// Steps follow BFS on the call graph from the domain entry symbol.
    pub fn reconstruct(symbols: &[SymbolRecord], edges: &[CallEdge]) -> Vec<FlowRecord> {
        let domains = discover_domains(symbols);
        let mut flows = Vec::new();

        for domain in domains {
            if let Some(flow) = Self::build_flow(&domain, symbols, edges) {
                flows.push(flow);
            }
        }

        flows.sort_by(|a, b| a.name.cmp(&b.name));
        flows
    }

    fn build_flow(
        domain: &str,
        symbols: &[SymbolRecord],
        edges: &[CallEdge],
    ) -> Option<FlowRecord> {
        let domain_ids: HashSet<String> = symbols
            .iter()
            .filter(|s| path_matches_domain(&s.file_path, domain))
            .map(|s| s.id.clone())
            .collect();
        Self::build_flow_for_symbol_set(
            &stable_flow_id(domain),
            domain,
            &domain_ids,
            symbols,
            edges,
        )
    }

    /// Builds a flow from an explicit symbol set (used by `domain add`).
    pub fn build_flow_for_symbol_set(
        flow_id: &str,
        name: &str,
        domain_ids: &HashSet<String>,
        symbols: &[SymbolRecord],
        edges: &[CallEdge],
    ) -> Option<FlowRecord> {
        let domain_symbols: Vec<&SymbolRecord> = symbols
            .iter()
            .filter(|s| domain_ids.contains(&s.id))
            .collect();

        if domain_symbols.len() < 2 {
            return None;
        }

        let allowed: HashSet<&str> = domain_symbols.iter().map(|s| s.id.as_str()).collect();

        let root = domain_symbols
            .iter()
            .find(|s| s.name == "main")
            .or_else(|| {
                domain_symbols
                    .iter()
                    .filter(|s| {
                        matches!(
                            s.kind,
                            repoctx_schema::symbol::SymbolKind::Function
                                | repoctx_schema::symbol::SymbolKind::Method
                        )
                    })
                    .min_by_key(|s| (s.file_path.as_str(), s.start_line))
            })?;

        let adjacency = build_adjacency(edges);
        let ordered_ids = bfs_order(root.id.as_str(), &adjacency, &allowed);

        let steps: Vec<FlowStepRecord> = ordered_ids
            .into_iter()
            .enumerate()
            .map(|(order, symbol_id)| FlowStepRecord {
                order: order as u32,
                symbol_id,
                external_system: None,
            })
            .collect();

        if steps.len() < 2 {
            return None;
        }

        Some(FlowRecord {
            id: flow_id.to_string(),
            name: name.to_string(),
            description: Some(format!("Flow for domain '{name}'")),
            steps,
        })
    }

    /// Resolves domain members to symbol ids.
    pub fn symbols_for_members(
        members: &[(String, String)],
        symbols: &[SymbolRecord],
    ) -> HashSet<String> {
        let mut ids = HashSet::new();
        for (kind, value) in members {
            match kind.as_str() {
                "path" => {
                    for symbol in symbols {
                        if path_matches_pattern(&symbol.file_path, value) {
                            ids.insert(symbol.id.clone());
                        }
                    }
                }
                "symbol" => {
                    for symbol in symbols {
                        if symbol.name == *value {
                            ids.insert(symbol.id.clone());
                        }
                    }
                }
                _ => {}
            }
        }
        ids
    }

    /// Builds a flow from stored domain members.
    pub fn build_flow_for_members(
        flow_id: &str,
        name: &str,
        members: &[(String, String)],
        symbols: &[SymbolRecord],
        edges: &[CallEdge],
    ) -> Option<FlowRecord> {
        let domain_ids = Self::symbols_for_members(members, symbols);
        Self::build_flow_for_symbol_set(flow_id, name, &domain_ids, symbols, edges)
    }
}

fn discover_domains(symbols: &[SymbolRecord]) -> Vec<String> {
    const SKIP: &[&str] = &[
        "src", "lib", "bin", "app", "tests", "test", "spec", "crates", "pkg", "internal", "cmd",
        "main", "mod", "common", "utils", "util", "core",
    ];

    let mut counts: HashMap<String, usize> = HashMap::new();
    for symbol in symbols {
        let parts: Vec<&str> = symbol.file_path.split('/').collect();
        if parts.len() < 2 {
            continue;
        }
        for segment in &parts[..parts.len() - 1] {
            let lower = segment.to_lowercase();
            if lower.len() >= 3 && !SKIP.contains(&lower.as_str()) {
                *counts.entry(lower).or_insert(0) += 1;
            }
        }
    }

    let mut domains: Vec<String> = counts
        .into_iter()
        .filter(|(_, count)| *count >= 2)
        .map(|(name, _)| name)
        .collect();
    domains.sort();
    domains
}

fn path_matches_domain(file_path: &str, domain: &str) -> bool {
    file_path
        .split('/')
        .any(|segment| segment.eq_ignore_ascii_case(domain))
}

fn path_matches_pattern(file_path: &str, pattern: &str) -> bool {
    let pattern = pattern.trim();
    if pattern.ends_with("/**") {
        let prefix = pattern.trim_end_matches("/**");
        return file_path == prefix || file_path.starts_with(&format!("{prefix}/"));
    }
    if pattern.contains('*') {
        let prefix = pattern.split('*').next().unwrap_or(pattern);
        return file_path.starts_with(prefix);
    }
    file_path == pattern || file_path.starts_with(&format!("{pattern}/"))
}

fn build_adjacency(edges: &[CallEdge]) -> HashMap<&str, Vec<&str>> {
    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in edges {
        adjacency
            .entry(edge.src.as_str())
            .or_default()
            .push(edge.dst.as_str());
    }
    for neighbors in adjacency.values_mut() {
        neighbors.sort_unstable();
    }
    adjacency
}

fn bfs_order(
    root_id: &str,
    adjacency: &HashMap<&str, Vec<&str>>,
    allowed: &HashSet<&str>,
) -> Vec<String> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut order = Vec::new();

    if allowed.contains(root_id) {
        queue.push_back(root_id);
        visited.insert(root_id);
    }

    while let Some(current) = queue.pop_front() {
        order.push(current.to_string());
        if let Some(neighbors) = adjacency.get(current) {
            for next in neighbors {
                if allowed.contains(next) && visited.insert(next) {
                    queue.push_back(next);
                }
            }
        }
    }

    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use repoctx_schema::symbol::{SymbolKind, Visibility};

    fn sym(id: &str, name: &str, path: &str) -> SymbolRecord {
        SymbolRecord {
            id: id.into(),
            kind: SymbolKind::Function,
            name: name.into(),
            fqn: format!("{path}::{name}"),
            file_path: path.into(),
            start_line: 1,
            end_line: 1,
            visibility: Visibility::Public,
            module_id: None,
        }
    }

    #[test]
    fn discovers_payment_domain_from_path() {
        let symbols = vec![
            sym("1", "checkout", "src/payment/checkout.rs"),
            sym("2", "charge", "src/payment/gateway.rs"),
        ];
        let domains = discover_domains(&symbols);
        assert!(domains.contains(&"payment".to_string()));
    }

    #[test]
    fn ignores_single_file_domains() {
        let symbols = vec![
            sym("a", "func_a", "src/graph.rs"),
            sym("b", "func_b", "src/graph.rs"),
        ];
        let domains = discover_domains(&symbols);
        assert!(domains.is_empty());
    }

    #[test]
    fn builds_flow_steps_in_call_order() {
        let symbols = vec![
            sym("a", "checkout", "src/payment/checkout.rs"),
            sym("b", "charge", "src/payment/checkout.rs"),
            sym("c", "validate", "src/payment/gateway.rs"),
        ];
        let edges = vec![
            CallEdge {
                src: "a".into(),
                dst: "b".into(),
            },
            CallEdge {
                src: "b".into(),
                dst: "c".into(),
            },
        ];
        let flows = FlowReconstructor::reconstruct(&symbols, &edges);
        let payment = flows
            .iter()
            .find(|f| f.name == "payment")
            .expect("payment flow");
        assert!(payment.steps.len() >= 2);
        assert_eq!(payment.id, stable_flow_id("payment"));
    }
}
