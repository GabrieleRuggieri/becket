//! ONNX-backed embeddings with deterministic hash fallback.

use crate::embedder::embed_text;

#[cfg(feature = "onnx")]
pub use crate::model::{
    default_model_cache_dir, hash_embed_forced, preload_onnx_model, try_embed_onnx,
    EMBED_CACHE_ENV, HASH_EMBED_ENV, ONNX_MODEL_ENV,
};

#[cfg(not(feature = "onnx"))]
pub const ONNX_MODEL_ENV: &str = "BECKET_ONNX_MODEL";

#[cfg(not(feature = "onnx"))]
pub const HASH_EMBED_ENV: &str = "BECKET_HASH_EMBED";

#[cfg(not(feature = "onnx"))]
pub const EMBED_CACHE_ENV: &str = "BECKET_EMBED_CACHE";

/// Embeds text with ONNX when available, otherwise falls back to deterministic hashing.
pub fn embed_with_model(text: &str) -> Vec<f32> {
    #[cfg(feature = "onnx")]
    {
        if let Some(vector) = try_embed_onnx(text) {
            return vector;
        }
    }

    #[cfg(not(feature = "onnx"))]
    if let Some(path) = std::env::var(ONNX_MODEL_ENV)
        .ok()
        .filter(|value| !value.is_empty())
    {
        tracing::info!(
            path = %path,
            "ONNX model path set but `onnx` feature disabled; using hash embedder"
        );
    }

    embed_text(text)
}

#[cfg(not(feature = "onnx"))]
pub fn preload_onnx_model() {}

#[cfg(not(feature = "onnx"))]
pub fn hash_embed_forced() -> bool {
    false
}
