//! Deterministic feature-hash embeddings (BGE-small compatible dimension).

use sha2::{Digest, Sha256};

/// Vector width aligned with BGE-small-en-v1.5.
pub const EMBEDDING_DIM: usize = 384;

/// Embedding dimension type alias.
pub type EmbeddingDim = usize;

/// Embeds arbitrary text into a normalized `EMBEDDING_DIM` vector.
///
/// Uses deterministic feature hashing so rebuilds produce identical vectors
/// without requiring a downloaded ONNX model. When `BECKET_ONNX_MODEL` is set
/// and the `onnx` feature is enabled, [`super::onnx::try_embed_onnx`] may be used
/// by callers instead.
pub fn embed_text(text: &str) -> Vec<f32> {
    let mut vec = vec![0f32; EMBEDDING_DIM];
    let lower = text.to_lowercase();
    let tokens: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|t| !t.is_empty())
        .collect();

    for token in &tokens {
        accumulate_token(&mut vec, token, 1.0);
    }
    for window in tokens.windows(2) {
        let bigram = format!("{}_{}", window[0], window[1]);
        accumulate_token(&mut vec, &bigram, 0.5);
    }

    l2_normalize(&mut vec);
    vec
}

fn accumulate_token(vec: &mut [f32], token: &str, weight: f32) {
    let hash = Sha256::digest(token.as_bytes());
    let idx = u32::from_le_bytes(hash[0..4].try_into().expect("4 bytes")) as usize % vec.len();
    vec[idx] += weight;
    let idx2 = u32::from_le_bytes(hash[4..8].try_into().expect("4 bytes")) as usize % vec.len();
    vec[idx2] += weight * 0.5;
}

pub(crate) fn l2_normalize(vec: &mut [f32]) {
    let norm: f32 = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        for v in vec.iter_mut() {
            *v /= norm;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embeddings_are_deterministic() {
        let a = embed_text("process_payment capture funds");
        let b = embed_text("process_payment capture funds");
        assert_eq!(a, b);
        assert_eq!(a.len(), EMBEDDING_DIM);
    }

    #[test]
    fn different_text_produces_different_vectors() {
        let a = embed_text("UserService");
        let b = embed_text("OrderRepository");
        assert_ne!(a, b);
    }
}
