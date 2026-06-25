//! Symbol text embedding for semantic search (deterministic hash or optional ONNX).

mod embedder;
#[cfg(feature = "onnx")]
mod model;
mod onnx;
mod text;
pub use embedder::{embed_text, EmbeddingDim, EMBEDDING_DIM};
#[cfg(feature = "onnx")]
pub use onnx::default_model_cache_dir;
pub use onnx::{
    embed_with_model, hash_embed_forced, preload_onnx_model, EMBED_CACHE_ENV, HASH_EMBED_ENV,
    ONNX_MODEL_ENV,
};
pub use text::symbol_embedding_text;
