/// Int8 quantized model utilities.
///
/// Placeholder for ONNX Runtime integration with quantized MiniLM.
/// In production, this handles:
/// - Model download and caching
/// - Int8 quantization verification
/// - Runtime optimization settings

use std::path::{Path, PathBuf};

/// Model metadata
pub struct ModelInfo {
    pub name: String,
    pub dim: usize,
    pub quantization: Quantization,
    pub size_bytes: u64,
    pub path: PathBuf,
}

pub enum Quantization {
    Float32,
    Float16,
    Int8,
}

impl ModelInfo {
    /// Default MiniLM-L6-v2 int8 quantized model
    pub fn minilm_int8(model_dir: impl AsRef<Path>) -> Self {
        Self {
            name: "all-MiniLM-L6-v2".to_string(),
            dim: 384,
            quantization: Quantization::Int8,
            size_bytes: 23_000_000, // ~23MB
            path: model_dir.as_ref().join("all-MiniLM-L6-v2-int8.onnx"),
        }
    }

    /// Check if the model file exists
    pub fn exists(&self) -> bool {
        self.path.exists()
    }
}

/// Check if the ONNX model is available, and return info about it
pub fn check_model(model_dir: impl AsRef<Path>) -> Option<ModelInfo> {
    let info = ModelInfo::minilm_int8(model_dir);
    if info.exists() {
        Some(info)
    } else {
        None
    }
}
