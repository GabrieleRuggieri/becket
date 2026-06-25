//! Embedding generation during `becket build`.

use becket_embed::{embed_with_model, preload_onnx_model, symbol_embedding_text};
use becket_schema::artifacts::SymbolRecord;
use becket_store::IndexStore;
use tracing::info;

use crate::error::CoreError;

/// Indexes symbol embeddings into sqlite-vec.
pub fn index_symbol_embeddings(
    store: &IndexStore,
    symbols: &[SymbolRecord],
) -> Result<usize, CoreError> {
    preload_onnx_model();

    let mut count = 0usize;
    for symbol in symbols {
        let text = symbol_embedding_text(symbol);
        let vector = embed_with_model(&text);
        store.upsert_symbol_embedding(&symbol.id, &vector)?;
        count += 1;
    }
    info!(count, "indexed symbol embeddings");
    Ok(count)
}
