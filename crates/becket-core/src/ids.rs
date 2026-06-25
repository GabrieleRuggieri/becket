//! Deterministic stable identifiers for symbols, edges, and flows.
//!
//! Architecture requires byte-identical artifacts across rebuilds on unchanged input.
//! IDs are derived from SHA-256 over canonical content keys — never random UUIDs.

use sha2::{Digest, Sha256};

/// Builds a stable symbol id from file path, name, start line, and kind.
pub fn stable_symbol_id(file_path: &str, name: &str, start_line: u32, kind: &str) -> String {
    stable_id(&["symbol", file_path, name, &start_line.to_string(), kind])
}

/// Builds a stable edge id from source, target, and edge type.
pub fn stable_edge_id(src: &str, dst: &str, edge_type: &str) -> String {
    stable_id(&["edge", src, dst, edge_type])
}

/// Builds a stable flow id from the flow name.
pub fn stable_flow_id(name: &str) -> String {
    stable_id(&["flow", name])
}

/// Builds a stable entrypoint id from symbol id and kind.
pub fn stable_entrypoint_id(symbol_id: &str, kind: &str) -> String {
    stable_id(&["entrypoint", symbol_id, kind])
}

/// Builds a stable file record id from repository-relative path.
pub fn stable_file_id(path: &str) -> String {
    stable_id(&["file", path])
}

/// Builds a stable cross-repo edge id.
pub fn stable_cross_repo_edge_id(
    src_repo: &str,
    src_symbol_id: &str,
    dst_repo: &str,
    dst_symbol_id: &str,
    edge_type: &str,
) -> String {
    stable_id(&[
        "cross_edge",
        src_repo,
        src_symbol_id,
        dst_repo,
        dst_symbol_id,
        edge_type,
    ])
}

fn stable_id(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    let digest = hasher.finalize();
    hex_encode(&digest[..16])
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_stable_across_calls() {
        let a = stable_symbol_id("src/main.rs", "main", 1, "function");
        let b = stable_symbol_id("src/main.rs", "main", 1, "function");
        assert_eq!(a, b);
        assert_ne!(a, stable_symbol_id("src/main.rs", "main", 2, "function"));
    }
}
