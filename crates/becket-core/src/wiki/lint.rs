//! Deterministic wiki lint (staleness, claims, links, orphans).

use std::collections::{HashMap, HashSet};
use std::fs;

use becket_schema::wiki::{WikiClaimError, WikiLinkError, WikiLintArtifact, WikiStaleQueue};
use becket_store::{BecketPaths, IndexStore};
use regex::Regex;

use crate::error::CoreError;
use crate::wiki::fingerprint::subgraph_fingerprint;
use crate::wiki::store::WikiStore;

/// Lints grounded wiki pages against the live graph.
pub struct WikiLinter {
    paths: BecketPaths,
}

impl WikiLinter {
    /// Creates a linter for the repository at `paths`.
    pub fn new(paths: BecketPaths) -> Self {
        Self { paths }
    }

    /// Runs full lint, writes `wiki_lint.json` and updates `wiki_stale.json`.
    pub fn run(&self, store: &IndexStore) -> Result<WikiLintArtifact, CoreError> {
        let wiki_store = WikiStore::new(&self.paths);
        let mut report = WikiLintArtifact::default();
        let call_edges = store.load_call_edges()?;
        let edge_set: HashSet<(String, String)> = call_edges.into_iter().collect();
        let known_ids: HashSet<String> = wiki_store.list_page_ids()?.into_iter().collect();

        let claim_re = Regex::new(r"<!--\s*becket:claim\s+(\w+)\s+(\S+)").unwrap();

        for page_id in &known_ids {
            let Some(page) = wiki_store.load_page(page_id)? else {
                continue;
            };
            let live_fp = subgraph_fingerprint(store, &page.meta.symbol_ids)?;
            if page.meta.graph_fingerprint != live_fp {
                report.stale_page_ids.push(page.meta.id.clone());
            }

            let anchors: HashSet<&str> = page.meta.symbol_ids.iter().map(|s| s.as_str()).collect();

            for cap in claim_re.captures_iter(&page.body) {
                let claim_type = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let target = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                if claim_type == "calls" && !claim_calls_valid(&anchors, target, &edge_set) {
                    report.claim_errors.push(WikiClaimError {
                        page_id: page.meta.id.clone(),
                        claim: format!("calls {target}"),
                        message: "no direct call edge between anchored symbols and target".into(),
                    });
                }
            }

            for link in &page.meta.see_also {
                if !known_ids.contains(link) && link != "wiki_index" {
                    report.broken_links.push(WikiLinkError {
                        page_id: page.meta.id.clone(),
                        target: link.clone(),
                        message: "see_also target not found".into(),
                    });
                }
            }
        }

        let inbound: HashMap<String, usize> = count_inbound_links(&wiki_store)?;
        for page_id in &known_ids {
            if inbound.get(page_id).copied().unwrap_or(0) == 0 {
                report.orphan_page_ids.push(page_id.clone());
            }
        }

        report.stale_page_ids.sort();
        report.orphan_page_ids.sort();

        let lint_path = self.paths.wiki_lint_report();
        let json =
            serde_json::to_string_pretty(&report).map_err(|e| CoreError::Wiki(e.to_string()))?;
        fs::write(lint_path, json)?;

        let queue = WikiStaleQueue {
            page_ids: report.stale_page_ids.clone(),
            ..WikiStaleQueue::default()
        };
        let queue_path = self.paths.wiki_stale_queue();
        let queue_json =
            serde_json::to_string_pretty(&queue).map_err(|e| CoreError::Wiki(e.to_string()))?;
        fs::write(queue_path, queue_json)?;

        Ok(report)
    }
}

/// Verifies a `calls TARGET` claim against direct edges involving anchored symbols.
pub fn claim_calls_valid(
    anchors: &HashSet<&str>,
    target: &str,
    edges: &HashSet<(String, String)>,
) -> bool {
    anchors.iter().any(|anchor| {
        edges.contains(&(anchor.to_string(), target.to_string()))
            || edges.contains(&(target.to_string(), anchor.to_string()))
    })
}

fn count_inbound_links(store: &WikiStore) -> Result<HashMap<String, usize>, CoreError> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    if let Some(index) = store.load_index()? {
        for link in &index.meta.see_also {
            *counts.entry(link.clone()).or_default() += 1;
        }
    }
    for page_id in store.list_page_ids()? {
        let Some(page) = store.load_page(&page_id)? else {
            continue;
        };
        for link in &page.meta.see_also {
            *counts.entry(link.clone()).or_default() += 1;
        }
    }
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claim_valid_only_for_direct_edges() {
        let anchors: HashSet<&str> = ["a"].into_iter().collect();
        let mut edges = HashSet::new();
        edges.insert(("a".into(), "b".into()));
        assert!(claim_calls_valid(&anchors, "b", &edges));
        assert!(!claim_calls_valid(&anchors, "c", &edges));
    }

    #[test]
    fn claim_accepts_reverse_edge() {
        let anchors: HashSet<&str> = ["svc"].into_iter().collect();
        let mut edges = HashSet::new();
        edges.insert(("caller".into(), "svc".into()));
        assert!(claim_calls_valid(&anchors, "caller", &edges));
    }
}
