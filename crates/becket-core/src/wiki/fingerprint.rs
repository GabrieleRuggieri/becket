//! Subgraph fingerprint for wiki staleness detection.

use std::collections::HashSet;

use becket_store::IndexStore;
use sha2::{Digest, Sha256};

use crate::error::CoreError;

/// Computes a stable hex fingerprint for `symbol_ids` and their incident call edges.
pub fn subgraph_fingerprint(
    store: &IndexStore,
    symbol_ids: &[String],
) -> Result<String, CoreError> {
    let edges = store.load_call_edges()?;
    let anchored: HashSet<&str> = symbol_ids.iter().map(String::as_str).collect();
    let mut parts: Vec<String> = symbol_ids.to_vec();
    parts.sort();
    for (src, dst) in edges {
        if anchored.contains(src.as_str()) || anchored.contains(dst.as_str()) {
            parts.push(format!("{src}->{dst}"));
        }
    }
    parts.sort();
    let mut hasher = Sha256::new();
    hasher.update(parts.join("\n").as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}
