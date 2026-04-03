use crate::memory::types::{Embedding, EngramResult};
use std::path::Path;

/// Embedding engine trait for generating vector representations of text.
pub trait EmbeddingEngine: Send + Sync {
    /// Generate an embedding for a text string
    fn embed(&self, text: &str) -> EngramResult<Embedding>;

    /// Generate embeddings for multiple texts (batched)
    fn embed_batch(&self, texts: &[&str]) -> EngramResult<Vec<Embedding>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    /// Dimension of the output embeddings
    fn dimension(&self) -> usize;
}

/// Simple hash-based embedding for testing and fallback.
/// Produces deterministic 384-dim vectors from text content.
/// NOT semantically meaningful — only for structural testing.
pub struct HashEmbeddingEngine {
    dim: usize,
}

impl HashEmbeddingEngine {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }

    pub fn default_384() -> Self {
        Self::new(384)
    }
}

impl EmbeddingEngine for HashEmbeddingEngine {
    fn embed(&self, text: &str) -> EngramResult<Embedding> {
        let mut embedding = vec![0.0f32; self.dim];
        let bytes = text.as_bytes();

        // Simple deterministic hash embedding
        for (i, &byte) in bytes.iter().enumerate() {
            let idx = (byte as usize * 7 + i * 13) % self.dim;
            embedding[idx] += 1.0;
        }

        // Normalize
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut embedding {
                *val /= norm;
            }
        }

        Ok(embedding)
    }

    fn dimension(&self) -> usize {
        self.dim
    }
}

// ============================================================================
// ONNX Embedding Engine (feature-gated)
// ============================================================================

/// Real ONNX embedding engine using all-MiniLM-L6-v2.
///
/// Provides semantically meaningful 384-dimensional embeddings.
/// "database" and "PostgreSQL" will be close neighbors.
/// "database" and "hiking" will be far apart.
///
/// Requires the `onnx` feature flag and model files at runtime.
#[cfg(feature = "onnx")]
pub struct OnnxEmbeddingEngine {
    session: parking_lot::Mutex<ort::session::Session>,
    tokenizer: tokenizers::Tokenizer,
    dim: usize,
    max_seq_len: usize,
}

#[cfg(feature = "onnx")]
impl OnnxEmbeddingEngine {
    /// Create from an ONNX model file and tokenizer.json
    pub fn new(model_path: impl AsRef<Path>, tokenizer_path: impl AsRef<Path>) -> EngramResult<Self> {
        let session = ort::session::Session::builder()
            .map_err(|e| EngramError::Embedding(format!("ONNX session builder: {}", e)))?
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)
            .map_err(|e| EngramError::Embedding(format!("Optimization level: {}", e)))?
            .commit_from_file(model_path.as_ref())
            .map_err(|e| EngramError::Embedding(format!("Load ONNX model: {}", e)))?;

        let tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path.as_ref())
            .map_err(|e| EngramError::Embedding(format!("Load tokenizer: {}", e)))?;

        Ok(Self {
            session: parking_lot::Mutex::new(session),
            tokenizer,
            dim: 384,
            max_seq_len: 128,
        })
    }

    /// Auto-discover model files from a directory
    pub fn from_dir(model_dir: impl AsRef<Path>) -> EngramResult<Self> {
        let dir = model_dir.as_ref();

        let model_names = ["model.onnx", "model_quantized.onnx", "all-MiniLM-L6-v2.onnx"];
        let model_path = model_names
            .iter()
            .map(|n| dir.join(n))
            .find(|p| p.exists())
            .ok_or_else(|| EngramError::Embedding(
                format!("No ONNX model found in {}. Expected one of: {:?}", dir.display(), model_names)
            ))?;

        let tokenizer_path = dir.join("tokenizer.json");
        if !tokenizer_path.exists() {
            return Err(EngramError::Embedding(
                format!("tokenizer.json not found in {}", dir.display())
            ));
        }

        Self::new(model_path, tokenizer_path)
    }

    /// Check if model files exist at the default location
    pub fn default_model_dir() -> std::path::PathBuf {
        std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("~"))
            .join(".localcode")
            .join("models")
            .join("embeddings")
    }

    /// Try to load from default location
    pub fn try_default() -> EngramResult<Self> {
        let dir = Self::default_model_dir();
        if !dir.exists() {
            return Err(EngramError::Embedding(
                "Embedding model not downloaded. Run model download first.".into()
            ));
        }
        Self::from_dir(dir)
    }

    fn run_inference(&self, text: &str) -> EngramResult<Embedding> {
        // Tokenize
        let encoding = self.tokenizer
            .encode(text, true)
            .map_err(|e| EngramError::Embedding(format!("Tokenization failed: {}", e)))?;

        let ids = encoding.get_ids();
        let attention_mask = encoding.get_attention_mask();
        let token_type_ids = encoding.get_type_ids();

        // Truncate to max_seq_len
        let seq_len = ids.len().min(self.max_seq_len);

        let input_ids: Vec<i64> = ids[..seq_len].iter().map(|&x| x as i64).collect();
        let attn_mask: Vec<i64> = attention_mask[..seq_len].iter().map(|&x| x as i64).collect();
        let type_ids: Vec<i64> = token_type_ids[..seq_len].iter().map(|&x| x as i64).collect();

        // Create ort Tensors with shape [1, seq_len]
        let shape = vec![1i64, seq_len as i64];

        let input_ids_tensor = ort::value::Tensor::from_array((shape.clone(), input_ids))
            .map_err(|e| EngramError::Embedding(format!("Tensor creation: {}", e)))?;
        let attn_mask_tensor = ort::value::Tensor::from_array((shape.clone(), attn_mask))
            .map_err(|e| EngramError::Embedding(format!("Tensor creation: {}", e)))?;
        let type_ids_tensor = ort::value::Tensor::from_array((shape, type_ids))
            .map_err(|e| EngramError::Embedding(format!("Tensor creation: {}", e)))?;

        // Run inference — session.run needs &mut self in ort 2.0
        let mut session = self.session.lock();
        let outputs = session.run(ort::inputs![
            "input_ids" => input_ids_tensor,
            "attention_mask" => attn_mask_tensor,
            "token_type_ids" => type_ids_tensor,
        ])
        .map_err(|e| EngramError::Embedding(format!("ONNX inference: {}", e)))?;

        // Extract output tensor: shape [1, seq_len, 384]
        let output_value = &outputs["last_hidden_state"];
        let (output_shape, output_data) = output_value
            .try_extract_tensor::<f32>()
            .map_err(|e| EngramError::Embedding(format!("Extract tensor: {}", e)))?;

        let out_dim = if output_shape.len() == 3 {
            output_shape[2] as usize
        } else {
            self.dim
        };

        // Mean pooling over token dimension, respecting attention mask
        let mut pooled = vec![0.0f32; out_dim];
        let mut mask_sum = 0.0f32;

        for t in 0..seq_len {
            let mask_val = encoding.get_attention_mask()[t] as f32;
            mask_sum += mask_val;
            let offset = t * out_dim;
            for d in 0..out_dim {
                if offset + d < output_data.len() {
                    pooled[d] += output_data[offset + d] * mask_val;
                }
            }
        }

        if mask_sum > 0.0 {
            for d in 0..out_dim {
                pooled[d] /= mask_sum;
            }
        }

        // L2 normalize
        let norm: f32 = pooled.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut pooled {
                *val /= norm;
            }
        }

        Ok(pooled)
    }
}

#[cfg(feature = "onnx")]
impl EmbeddingEngine for OnnxEmbeddingEngine {
    fn embed(&self, text: &str) -> EngramResult<Embedding> {
        self.run_inference(text)
    }

    fn embed_batch(&self, texts: &[&str]) -> EngramResult<Vec<Embedding>> {
        texts.iter().map(|t| self.run_inference(t)).collect()
    }

    fn dimension(&self) -> usize {
        self.dim
    }
}

// ============================================================================
// Stub when ONNX feature is not enabled
// ============================================================================

#[cfg(not(feature = "onnx"))]
pub struct OnnxEmbeddingEngine {
    dim: usize,
}

#[cfg(not(feature = "onnx"))]
impl OnnxEmbeddingEngine {
    pub fn new(_model_path: impl AsRef<Path>, _tokenizer_path: impl AsRef<Path>) -> EngramResult<Self> {
        tracing::warn!("ONNX feature not enabled — using hash embeddings. Enable with --features onnx");
        Ok(Self { dim: 384 })
    }

    pub fn from_dir(_model_dir: impl AsRef<Path>) -> EngramResult<Self> {
        tracing::warn!("ONNX feature not enabled — using hash embeddings");
        Ok(Self { dim: 384 })
    }

    pub fn try_default() -> EngramResult<Self> {
        Ok(Self { dim: 384 })
    }
}

#[cfg(not(feature = "onnx"))]
impl EmbeddingEngine for OnnxEmbeddingEngine {
    fn embed(&self, text: &str) -> EngramResult<Embedding> {
        HashEmbeddingEngine::new(self.dim).embed(text)
    }

    fn dimension(&self) -> usize {
        self.dim
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_embedding() {
        let engine = HashEmbeddingEngine::default_384();
        let emb = engine.embed("hello world").unwrap();

        assert_eq!(emb.len(), 384);

        // Should be normalized
        let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01, "Embedding should be unit-normalized, got norm {}", norm);
    }

    #[test]
    fn test_deterministic() {
        let engine = HashEmbeddingEngine::default_384();
        let emb1 = engine.embed("test string").unwrap();
        let emb2 = engine.embed("test string").unwrap();

        assert_eq!(emb1, emb2, "Same input should produce same embedding");
    }

    #[test]
    fn test_different_inputs() {
        let engine = HashEmbeddingEngine::default_384();
        let emb1 = engine.embed("hello").unwrap();
        let emb2 = engine.embed("world").unwrap();

        assert_ne!(emb1, emb2, "Different inputs should produce different embeddings");
    }

    #[test]
    fn test_batch_embedding() {
        let engine = HashEmbeddingEngine::default_384();
        let embeddings = engine.embed_batch(&["hello", "world", "test"]).unwrap();

        assert_eq!(embeddings.len(), 3);
        assert_eq!(embeddings[0].len(), 384);
    }
}
