//! Optional ONNX runtime embeddings (BGE-small class) when a model file is present.

use std::path::Path;

use tracing::info;

use crate::embedder::embed_text;

/// Environment variable pointing to a local `.onnx` embedding model.
pub const ONNX_MODEL_ENV: &str = "REPOCTX_ONNX_MODEL";

/// Embeds text with ONNX when configured, otherwise falls back to deterministic hashing.
pub fn embed_with_model(text: &str) -> Vec<f32> {
    if let Some(path) = std::env::var(ONNX_MODEL_ENV)
        .ok()
        .filter(|value| !value.is_empty())
    {
        if let Some(vec) = try_embed_onnx(Path::new(&path), text) {
            return vec;
        }
    }
    embed_text(text)
}

/// Placeholder for ONNX inference — loads path check only until `ort` integration lands.
pub fn try_embed_onnx(model_path: &Path, text: &str) -> Option<Vec<f32>> {
    if !model_path.is_file() {
        return None;
    }
    info!(
        path = %model_path.display(),
        "ONNX model path set; tokenizer integration pending, using hash embedder"
    );
    let _ = text;
    None
}
