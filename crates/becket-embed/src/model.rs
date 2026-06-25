//! Lazy-loaded BGE-small ONNX embedder with Hugging Face model download + local cache.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use fastembed::{
    EmbeddingModel, InitOptionsUserDefined, TextEmbedding, TextInitOptions, TokenizerFiles,
    UserDefinedEmbeddingModel,
};
use tracing::{info, warn};

use crate::embedder::{l2_normalize, EMBEDDING_DIM};

/// When set to a truthy value, always use the deterministic hash embedder (CI / tests).
pub const HASH_EMBED_ENV: &str = "BECKET_HASH_EMBED";

/// Optional override for the ONNX model cache directory.
pub const EMBED_CACHE_ENV: &str = "BECKET_EMBED_CACHE";

/// Environment variable pointing to a local ONNX model directory or `.onnx` file.
pub const ONNX_MODEL_ENV: &str = "BECKET_ONNX_MODEL";

static ONNX_EMBEDDER: OnceLock<Mutex<Option<TextEmbedding>>> = OnceLock::new();

/// Returns true when hash embeddings are forced via environment.
pub fn hash_embed_forced() -> bool {
    std::env::var(HASH_EMBED_ENV)
        .ok()
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes"))
}

/// Default Hugging Face cache directory for Becket embedding models.
pub fn default_model_cache_dir() -> PathBuf {
    if let Ok(path) = std::env::var(EMBED_CACHE_ENV) {
        if !path.is_empty() {
            return PathBuf::from(path);
        }
    }
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".becket"))
        .join("becket")
        .join("models")
}

/// Preloads the ONNX model (downloads on first use). No-op when hash mode is forced.
pub fn preload_onnx_model() {
    if hash_embed_forced() {
        return;
    }
    let _ = onnx_embedder();
}

/// Embeds text with the bundled BGE-small model when available.
pub fn try_embed_onnx(text: &str) -> Option<Vec<f32>> {
    if hash_embed_forced() {
        return None;
    }

    let mut guard = onnx_embedder()?.lock().ok()?;
    let model = guard.as_mut()?;
    let mut vector = model.embed([text], None).ok()?.pop()?;
    if vector.len() != EMBEDDING_DIM {
        warn!(
            expected = EMBEDDING_DIM,
            actual = vector.len(),
            "ONNX embedding dimension mismatch, using hash fallback"
        );
        return None;
    }
    l2_normalize(&mut vector);
    Some(vector)
}

fn onnx_embedder() -> Option<&'static Mutex<Option<TextEmbedding>>> {
    Some(ONNX_EMBEDDER.get_or_init(|| match load_onnx_embedder() {
        Ok(model) => Mutex::new(Some(model)),
        Err(error) => {
            warn!(%error, "ONNX embedder unavailable, using hash fallback");
            Mutex::new(None)
        }
    }))
}

fn load_onnx_embedder() -> Result<TextEmbedding, String> {
    if let Some(path) = custom_model_path() {
        info!(path = %path.display(), "loading user-defined ONNX embedding model");
        return load_user_defined_model(&path);
    }

    let cache_dir = default_model_cache_dir();
    std::fs::create_dir_all(&cache_dir).map_err(|error| error.to_string())?;

    info!(
        cache = %cache_dir.display(),
        model = "BAAI/bge-small-en-v1.5",
        "initializing ONNX embedding model (downloads on first use)"
    );

    TextEmbedding::try_new(
        TextInitOptions::new(EmbeddingModel::BGESmallENV15)
            .with_cache_dir(cache_dir)
            .with_show_download_progress(true),
    )
    .map_err(|error| error.to_string())
}

fn custom_model_path() -> Option<PathBuf> {
    std::env::var(ONNX_MODEL_ENV)
        .ok()
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn load_user_defined_model(path: &Path) -> Result<TextEmbedding, String> {
    let model_dir = if path.is_dir() {
        path.to_path_buf()
    } else if path.extension().is_some_and(|ext| ext == "onnx") {
        path.parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| format!("invalid ONNX model path: {}", path.display()))?
    } else {
        return Err(format!(
            "BECKET_ONNX_MODEL must be a directory or .onnx file: {}",
            path.display()
        ));
    };

    let onnx_path = if path.is_file() {
        path.to_path_buf()
    } else {
        model_dir.join("model.onnx")
    };

    if !onnx_path.is_file() {
        return Err(format!(
            "ONNX model file not found at {}",
            onnx_path.display()
        ));
    }

    let tokenizer_files = TokenizerFiles {
        tokenizer_file: read_required(&model_dir.join("tokenizer.json"))?,
        config_file: read_required(&model_dir.join("config.json"))?,
        special_tokens_map_file: read_required(&model_dir.join("special_tokens_map.json"))?,
        tokenizer_config_file: read_required(&model_dir.join("tokenizer_config.json"))?,
    };

    let onnx_file = std::fs::read(&onnx_path).map_err(|error| error.to_string())?;
    let user_model = UserDefinedEmbeddingModel::new(onnx_file, tokenizer_files);

    TextEmbedding::try_new_from_user_defined(user_model, InitOptionsUserDefined::default())
        .map_err(|error| error.to_string())
}

fn read_required(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn env_lock() -> MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner())
    }

    #[test]
    fn default_cache_dir_is_under_system_cache() {
        let _guard = env_lock();
        std::env::remove_var(EMBED_CACHE_ENV);
        let dir = default_model_cache_dir();
        assert!(dir.to_string_lossy().contains("becket"));
        assert!(dir.to_string_lossy().ends_with("models"));
    }

    #[test]
    fn hash_embed_forced_reads_env() {
        let _guard = env_lock();
        std::env::set_var(HASH_EMBED_ENV, "1");
        assert!(hash_embed_forced());
        std::env::remove_var(HASH_EMBED_ENV);
    }

    #[test]
    fn try_embed_onnx_uses_hash_when_forced() {
        let _guard = env_lock();
        std::env::set_var(HASH_EMBED_ENV, "1");
        assert!(try_embed_onnx("hello").is_none());
        std::env::remove_var(HASH_EMBED_ENV);
    }

    #[test]
    fn embed_with_model_falls_back_to_hash_without_model() {
        let _guard = env_lock();
        std::env::set_var(HASH_EMBED_ENV, "1");
        let hash = crate::embedder::embed_text("payment service");
        let merged = crate::onnx::embed_with_model("payment service");
        assert_eq!(hash, merged);
        std::env::remove_var(HASH_EMBED_ENV);
    }
}
