//! Domain refinement: rename auto-discovered flows and attach path/symbol members.

use std::path::Path;

use repoctx_schema::artifacts::FlowRecord;
use repoctx_store::{DomainOverride, IndexStore, RepoCtxPaths};

use crate::error::CoreError;
use crate::flow::{CallEdge, FlowReconstructor};
use crate::ids::stable_flow_id;

/// Persists user domain refinements in the index and re-emits JSON artifacts.
pub struct DomainEditor {
    paths: RepoCtxPaths,
}

impl DomainEditor {
    /// Opens the editor for the repository at `root`.
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            paths: RepoCtxPaths::new(root),
        }
    }

    /// Renames an auto-discovered domain (by flow id or current name).
    pub fn rename(&self, auto_id: &str, new_name: &str) -> Result<FlowRecord, CoreError> {
        let store = self.open_store()?;
        let flow_id = store
            .find_flow_id(auto_id)?
            .ok_or_else(|| CoreError::Domain(format!("domain or flow '{auto_id}' not found")))?;

        if store.flow_name_taken(new_name, Some(&flow_id))? {
            return Err(CoreError::Domain(format!(
                "flow name '{new_name}' is already in use"
            )));
        }

        store.update_flow_name(&flow_id, new_name)?;
        store.upsert_domain(&flow_id, new_name, "user", true)?;
        store.write_artifacts(&self.paths)?;

        store
            .find_flow_by_name(new_name)?
            .ok_or_else(|| CoreError::Domain("flow missing after rename".into()))
    }

    /// Attaches path globs or symbol names to a domain and rebuilds its flow.
    pub fn add(&self, name: &str, targets: &[String]) -> Result<FlowRecord, CoreError> {
        if targets.is_empty() {
            return Err(CoreError::Domain(
                "provide at least one path or symbol target".into(),
            ));
        }

        let store = self.open_store()?;
        let flow_id = match store.find_flow_id(name)? {
            Some(id) => id,
            None => {
                if store.flow_name_taken(name, None)? {
                    return Err(CoreError::Domain(format!(
                        "flow name '{name}' is already in use"
                    )));
                }
                stable_flow_id(name)
            }
        };

        store.upsert_domain(&flow_id, name, "user", true)?;

        for target in targets {
            let kind = classify_target(target);
            store.add_domain_member(&flow_id, kind, target)?;
        }

        let flow = self.rebuild_flow(&store, &flow_id, name)?;
        store.write_artifacts(&self.paths)?;
        Ok(flow)
    }

    fn open_store(&self) -> Result<IndexStore, CoreError> {
        if !self.paths.index_db.exists() {
            return Err(CoreError::Domain(
                "index not found; run `repoctx build` first".into(),
            ));
        }
        Ok(IndexStore::open(&self.paths.index_db)?)
    }

    fn rebuild_flow(
        &self,
        store: &IndexStore,
        flow_id: &str,
        name: &str,
    ) -> Result<FlowRecord, CoreError> {
        let symbols = store.load_symbols()?;
        let call_edges: Vec<CallEdge> = store
            .load_call_edges()?
            .into_iter()
            .map(|(src, dst)| CallEdge { src, dst })
            .collect();
        let members = store.list_domain_members(flow_id)?;

        let flow = FlowReconstructor::build_flow_for_members(
            flow_id,
            name,
            &members,
            &symbols,
            &call_edges,
        )
        .ok_or_else(|| {
            CoreError::Domain(format!(
                "could not build flow '{name}' — need at least two matching symbols"
            ))
        })?;

        store.replace_flow(&flow)?;
        Ok(flow)
    }
}

fn classify_target(target: &str) -> &'static str {
    if target.contains('/') || target.contains('*') {
        "path"
    } else {
        "symbol"
    }
}

/// Applies user domain overrides after auto-discovery during `repoctx build`.
pub fn apply_domain_overrides(
    flows: &mut Vec<FlowRecord>,
    overrides: &[DomainOverride],
    symbols: &[repoctx_schema::artifacts::SymbolRecord],
    call_edges: &[CallEdge],
) {
    for (id, name, members) in overrides {
        if members.is_empty() {
            if let Some(flow) = flows.iter_mut().find(|f| f.id == *id) {
                flow.name.clone_from(name);
            }
            continue;
        }

        if let Some(flow) =
            FlowReconstructor::build_flow_for_members(id, name, members, symbols, call_edges)
        {
            if let Some(existing) = flows.iter_mut().find(|f| f.id == *id) {
                *existing = flow;
            } else {
                flows.push(flow);
            }
        } else if let Some(flow) = flows.iter_mut().find(|f| f.id == *id) {
            flow.name.clone_from(name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_path_and_symbol_targets() {
        assert_eq!(classify_target("src/payment/**"), "path");
        assert_eq!(classify_target("CheckoutService"), "symbol");
    }
}
