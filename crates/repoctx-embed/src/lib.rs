//! Symbol text embedding for semantic search (deterministic hash or optional ONNX).

mod embedder;
mod onnx;
mod text;
pub use embedder::{embed_text, EmbeddingDim, EMBEDDING_DIM};
pub use onnx::{embed_with_model, ONNX_MODEL_ENV};
pub use text::symbol_embedding_text;
