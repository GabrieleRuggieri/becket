//! Canonical text payloads fed into the embedder.

use repoctx_schema::artifacts::SymbolRecord;

/// Builds a stable, human-readable string representing a symbol for embedding.
pub fn symbol_embedding_text(symbol: &SymbolRecord) -> String {
    format!(
        "{} {:?} {} lines {}-{}",
        symbol.name, symbol.kind, symbol.file_path, symbol.start_line, symbol.end_line
    )
}
