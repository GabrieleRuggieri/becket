//! Embedding generation during `repoctx build`.

use repoctx_embed::{embed_with_model, symbol_embedding_text};
use repoctx_schema::artifacts::SymbolRecord;
use repoctx_store::IndexStore;
use tracing::info;

use crate::error::CoreError;

/// Indexes symbol embeddings into sqlite-vec.
pub fn index_symbol_embeddings(
    store: &IndexStore,
    symbols: &[SymbolRecord],
) -> Result<usize, CoreError> {
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
