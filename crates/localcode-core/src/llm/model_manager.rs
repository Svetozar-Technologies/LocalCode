use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::CoreResult;
use crate::CoreError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCatalogEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub url: String,
    pub filename: String,
    pub size_bytes: u64,
    pub quantization: String,
    pub context_length: u32,
    pub parameters: String,
    pub family: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadedModel {
    pub catalog_id: String,
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub downloaded_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub catalog_id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub speed_bps: u64,
    pub eta_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelRegistry {
    pub models: Vec<DownloadedModel>,
}

pub struct ModelManager {
    models_dir: PathBuf,
    registry_path: PathBuf,
    registry: ModelRegistry,
}

impl ModelManager {
    pub fn new() -> Self {
        let base_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".localcode")
            .join("models");

        let registry_path = base_dir.join("registry.json");
        let registry = Self::load_registry(&registry_path).unwrap_or_default();

        Self {
            models_dir: base_dir,
            registry_path,
            registry,
        }
    }

    fn load_registry(path: &Path) -> CoreResult<ModelRegistry> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let registry: ModelRegistry = serde_json::from_str(&content)?;
            Ok(registry)
        } else {
            Ok(ModelRegistry::default())
        }
    }

    fn save_registry(&self) -> CoreResult<()> {
        if let Some(parent) = self.registry_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.registry)?;
        std::fs::write(&self.registry_path, content)?;
        Ok(())
    }

    /// Curated catalog of best coding models
    pub fn catalog() -> Vec<ModelCatalogEntry> {
        vec![
            // Phi-3-mini — lightweight, great for low-resource machines
            ModelCatalogEntry {
                id: "phi-3-mini-4k-q4".to_string(),
                name: "Phi-3 Mini 3.8B".to_string(),
                description: "Microsoft's compact model. Fast on CPU, good for basic coding tasks.".to_string(),
                url: "https://huggingface.co/bartowski/Phi-3.1-mini-4k-instruct-GGUF/resolve/main/Phi-3.1-mini-4k-instruct-Q4_K_M.gguf".to_string(),
                filename: "Phi-3.1-mini-4k-instruct-Q4_K_M.gguf".to_string(),
                size_bytes: 2_394_715_040,
                quantization: "Q4_K_M".to_string(),
                context_length: 4096,
                parameters: "3.8B".to_string(),
                family: "phi".to_string(),
                tags: vec!["recommended".to_string(), "lightweight".to_string(), "fast".to_string()],
            },
            // Qwen2.5-Coder 7B Q4
            ModelCatalogEntry {
                id: "qwen2.5-coder-7b-q4".to_string(),
                name: "Qwen2.5-Coder 7B".to_string(),
                description: "Top-tier coding model from Alibaba. Excellent code completion and understanding.".to_string(),
                url: "https://huggingface.co/bartowski/Qwen2.5-Coder-7B-Instruct-GGUF/resolve/main/Qwen2.5-Coder-7B-Instruct-Q4_K_M.gguf".to_string(),
                filename: "Qwen2.5-Coder-7B-Instruct-Q4_K_M.gguf".to_string(),
                size_bytes: 4_683_218_944,
                quantization: "Q4_K_M".to_string(),
                context_length: 32768,
                parameters: "7B".to_string(),
                family: "qwen".to_string(),
                tags: vec!["recommended".to_string(), "coding".to_string()],
            },
            // Qwen2.5-Coder 7B Q8
            ModelCatalogEntry {
                id: "qwen2.5-coder-7b-q8".to_string(),
                name: "Qwen2.5-Coder 7B (Q8)".to_string(),
                description: "Higher quality quantization for best coding accuracy. Requires more RAM.".to_string(),
                url: "https://huggingface.co/bartowski/Qwen2.5-Coder-7B-Instruct-GGUF/resolve/main/Qwen2.5-Coder-7B-Instruct-Q8_0.gguf".to_string(),
                filename: "Qwen2.5-Coder-7B-Instruct-Q8_0.gguf".to_string(),
                size_bytes: 8_096_702_464,
                quantization: "Q8_0".to_string(),
                context_length: 32768,
                parameters: "7B".to_string(),
                family: "qwen".to_string(),
                tags: vec!["coding".to_string(), "high-quality".to_string()],
            },
            // Qwen2.5-Coder 14B Q4
            ModelCatalogEntry {
                id: "qwen2.5-coder-14b-q4".to_string(),
                name: "Qwen2.5-Coder 14B".to_string(),
                description: "Larger coding model with stronger reasoning. Needs 10GB+ RAM.".to_string(),
                url: "https://huggingface.co/bartowski/Qwen2.5-Coder-14B-Instruct-GGUF/resolve/main/Qwen2.5-Coder-14B-Instruct-Q4_K_M.gguf".to_string(),
                filename: "Qwen2.5-Coder-14B-Instruct-Q4_K_M.gguf".to_string(),
                size_bytes: 9_028_100_096,
                quantization: "Q4_K_M".to_string(),
                context_length: 32768,
                parameters: "14B".to_string(),
                family: "qwen".to_string(),
                tags: vec!["coding".to_string(), "large".to_string()],
            },
            // DeepSeek-Coder-V2-Lite 16B
            ModelCatalogEntry {
                id: "deepseek-coder-v2-lite-q4".to_string(),
                name: "DeepSeek-Coder-V2-Lite 16B".to_string(),
                description: "MoE architecture coding model. Strong multi-language code generation.".to_string(),
                url: "https://huggingface.co/bartowski/DeepSeek-Coder-V2-Lite-Instruct-GGUF/resolve/main/DeepSeek-Coder-V2-Lite-Instruct-Q4_K_M.gguf".to_string(),
                filename: "DeepSeek-Coder-V2-Lite-Instruct-Q4_K_M.gguf".to_string(),
                size_bytes: 9_033_564_160,
                quantization: "Q4_K_M".to_string(),
                context_length: 16384,
                parameters: "16B".to_string(),
                family: "deepseek".to_string(),
                tags: vec!["coding".to_string(), "moe".to_string()],
            },
            // StarCoder2 7B
            ModelCatalogEntry {
                id: "starcoder2-7b-q4".to_string(),
                name: "StarCoder2 7B".to_string(),
                description: "BigCode's code generation model. Trained on The Stack v2.".to_string(),
                url: "https://huggingface.co/bartowski/starcoder2-7b-GGUF/resolve/main/starcoder2-7b-Q4_K_M.gguf".to_string(),
                filename: "starcoder2-7b-Q4_K_M.gguf".to_string(),
                size_bytes: 4_370_000_000,
                quantization: "Q4_K_M".to_string(),
                context_length: 16384,
                parameters: "7B".to_string(),
                family: "starcoder".to_string(),
                tags: vec!["coding".to_string(), "completion".to_string()],
            },
            // StarCoder2 15B
            ModelCatalogEntry {
                id: "starcoder2-15b-q4".to_string(),
                name: "StarCoder2 15B".to_string(),
                description: "Larger StarCoder2 with better code understanding. Needs 12GB+ RAM.".to_string(),
                url: "https://huggingface.co/bartowski/starcoder2-15b-GGUF/resolve/main/starcoder2-15b-Q4_K_M.gguf".to_string(),
                filename: "starcoder2-15b-Q4_K_M.gguf".to_string(),
                size_bytes: 9_150_000_000,
                quantization: "Q4_K_M".to_string(),
                context_length: 16384,
                parameters: "15B".to_string(),
                family: "starcoder".to_string(),
                tags: vec!["coding".to_string(), "large".to_string()],
            },
            // CodeLlama 7B
            ModelCatalogEntry {
                id: "codellama-7b-q4".to_string(),
                name: "CodeLlama 7B".to_string(),
                description: "Meta's code-specialized Llama model. Good all-rounder for coding tasks.".to_string(),
                url: "https://huggingface.co/bartowski/CodeLlama-7b-Instruct-hf-GGUF/resolve/main/CodeLlama-7b-Instruct-hf-Q4_K_M.gguf".to_string(),
                filename: "CodeLlama-7b-Instruct-hf-Q4_K_M.gguf".to_string(),
                size_bytes: 4_081_004_480,
                quantization: "Q4_K_M".to_string(),
                context_length: 16384,
                parameters: "7B".to_string(),
                family: "codellama".to_string(),
                tags: vec!["coding".to_string()],
            },
            // CodeLlama 13B
            ModelCatalogEntry {
                id: "codellama-13b-q4".to_string(),
                name: "CodeLlama 13B".to_string(),
                description: "Larger CodeLlama with stronger code reasoning. Needs 10GB+ RAM.".to_string(),
                url: "https://huggingface.co/bartowski/CodeLlama-13b-Instruct-hf-GGUF/resolve/main/CodeLlama-13b-Instruct-hf-Q4_K_M.gguf".to_string(),
                filename: "CodeLlama-13b-Instruct-hf-Q4_K_M.gguf".to_string(),
                size_bytes: 7_866_000_000,
                quantization: "Q4_K_M".to_string(),
                context_length: 16384,
                parameters: "13B".to_string(),
                family: "codellama".to_string(),
                tags: vec!["coding".to_string(), "large".to_string()],
            },
        ]
    }

    /// Check if a model is downloaded
    pub fn is_downloaded(&self, catalog_id: &str) -> bool {
        self.registry
            .models
            .iter()
            .any(|m| m.catalog_id == catalog_id && Path::new(&m.path).exists())
    }

    /// Get path of a downloaded model
    pub fn get_model_path(&self, catalog_id: &str) -> Option<String> {
        self.registry
            .models
            .iter()
            .find(|m| m.catalog_id == catalog_id && Path::new(&m.path).exists())
            .map(|m| m.path.clone())
    }

    /// List all downloaded models
    pub fn list_downloaded(&self) -> Vec<DownloadedModel> {
        self.registry
            .models
            .iter()
            .filter(|m| Path::new(&m.path).exists())
            .cloned()
            .collect()
    }

    /// Download a model from the catalog with progress callback
    pub async fn download<F>(
        &mut self,
        catalog_id: &str,
        on_progress: F,
    ) -> CoreResult<String>
    where
        F: Fn(DownloadProgress) + Send + 'static,
    {
        let catalog = Self::catalog();
        let entry = catalog
            .iter()
            .find(|e| e.id == catalog_id)
            .ok_or_else(|| CoreError::Other(format!("Unknown model: {}", catalog_id)))?
            .clone();

        // Create models directory
        std::fs::create_dir_all(&self.models_dir)?;

        let dest_path = self.models_dir.join(&entry.filename);
        let dest_str = dest_path.to_string_lossy().to_string();

        // If already fully downloaded, return path
        if dest_path.exists() {
            let metadata = std::fs::metadata(&dest_path)?;
            if metadata.len() == entry.size_bytes {
                // Register if not already
                if !self.is_downloaded(catalog_id) {
                    self.registry.models.push(DownloadedModel {
                        catalog_id: catalog_id.to_string(),
                        path: dest_str.clone(),
                        name: entry.name.clone(),
                        size_bytes: entry.size_bytes,
                        downloaded_at: now_secs(),
                    });
                    self.save_registry()?;
                }
                return Ok(dest_str);
            }
        }

        // Streaming download with progress
        let client = reqwest::Client::new();

        // Check for partial download (resume support)
        let existing_bytes = if dest_path.exists() {
            std::fs::metadata(&dest_path).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        let mut request = client.get(&entry.url);
        if existing_bytes > 0 {
            request = request.header("Range", format!("bytes={}-", existing_bytes));
        }

        let response = request.send().await.map_err(|e| {
            CoreError::Other(format!("Download request failed: {}", e))
        })?;

        let status = response.status();
        if !status.is_success() && status.as_u16() != 206 {
            return Err(CoreError::Other(format!(
                "Download failed with HTTP {}",
                status
            )));
        }

        let total_bytes = if status.as_u16() == 206 {
            entry.size_bytes
        } else {
            response
                .content_length()
                .unwrap_or(entry.size_bytes)
        };

        use futures::StreamExt;
        use std::io::Write;

        let file = if existing_bytes > 0 && status.as_u16() == 206 {
            std::fs::OpenOptions::new()
                .append(true)
                .open(&dest_path)?
        } else {
            std::fs::File::create(&dest_path)?
        };
        let mut writer = std::io::BufWriter::new(file);

        let mut downloaded = if status.as_u16() == 206 {
            existing_bytes
        } else {
            0
        };
        let start_time = std::time::Instant::now();
        let mut stream = response.bytes_stream();
        let cid = catalog_id.to_string();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| CoreError::Other(format!("Download error: {}", e)))?;
            writer.write_all(&chunk)?;
            downloaded += chunk.len() as u64;

            let elapsed = start_time.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 {
                (downloaded as f64 / elapsed) as u64
            } else {
                0
            };
            let remaining = total_bytes.saturating_sub(downloaded);
            let eta = if speed > 0 { remaining / speed } else { 0 };

            on_progress(DownloadProgress {
                catalog_id: cid.clone(),
                downloaded_bytes: downloaded,
                total_bytes,
                speed_bps: speed,
                eta_seconds: eta,
            });
        }

        writer.flush()?;

        // Register the downloaded model
        // Remove old entries for same catalog_id
        self.registry.models.retain(|m| m.catalog_id != catalog_id);
        self.registry.models.push(DownloadedModel {
            catalog_id: catalog_id.to_string(),
            path: dest_str.clone(),
            name: entry.name,
            size_bytes: downloaded,
            downloaded_at: now_secs(),
        });
        self.save_registry()?;

        Ok(dest_str)
    }

    /// Delete a downloaded model
    pub fn delete_model(&mut self, catalog_id: &str) -> CoreResult<()> {
        if let Some(model) = self
            .registry
            .models
            .iter()
            .find(|m| m.catalog_id == catalog_id)
        {
            let path = Path::new(&model.path);
            if path.exists() {
                std::fs::remove_file(path)?;
            }
        }
        self.registry.models.retain(|m| m.catalog_id != catalog_id);
        self.save_registry()?;
        Ok(())
    }
}

impl Default for ModelManager {
    fn default() -> Self {
        Self::new()
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_catalog_not_empty() {
        let catalog = ModelManager::catalog();
        assert!(!catalog.is_empty(), "Model catalog should not be empty");
    }

    #[test]
    fn test_catalog_entries_valid() {
        let catalog = ModelManager::catalog();
        for entry in &catalog {
            assert!(!entry.id.is_empty(), "Catalog entry id should not be empty");
            assert!(!entry.name.is_empty(), "Catalog entry name should not be empty: {}", entry.id);
            assert!(!entry.url.is_empty(), "Catalog entry url should not be empty: {}", entry.id);
            assert!(!entry.filename.is_empty(), "Catalog entry filename should not be empty: {}", entry.id);
            assert!(entry.size_bytes > 0, "Catalog entry size should be > 0: {}", entry.id);
            assert!(entry.context_length > 0, "Catalog entry context_length should be > 0: {}", entry.id);
            assert!(!entry.quantization.is_empty(), "Catalog entry quantization should not be empty: {}", entry.id);
            assert!(!entry.parameters.is_empty(), "Catalog entry parameters should not be empty: {}", entry.id);
            assert!(!entry.family.is_empty(), "Catalog entry family should not be empty: {}", entry.id);
        }
    }

    #[test]
    fn test_registry_save_load() {
        let dir = TempDir::new().unwrap();
        let registry_path = dir.path().join("registry.json");

        let registry = ModelRegistry {
            models: vec![
                DownloadedModel {
                    catalog_id: "test-model-q4".to_string(),
                    path: "/tmp/test-model.gguf".to_string(),
                    name: "Test Model".to_string(),
                    size_bytes: 1_000_000,
                    downloaded_at: 1700000000,
                },
                DownloadedModel {
                    catalog_id: "another-model-q8".to_string(),
                    path: "/tmp/another-model.gguf".to_string(),
                    name: "Another Model".to_string(),
                    size_bytes: 2_000_000,
                    downloaded_at: 1700000001,
                },
            ],
        };

        // Save
        let content = serde_json::to_string_pretty(&registry).unwrap();
        std::fs::write(&registry_path, &content).unwrap();

        // Load
        let loaded = ModelManager::load_registry(&registry_path).unwrap();
        assert_eq!(loaded.models.len(), 2);
        assert_eq!(loaded.models[0].catalog_id, "test-model-q4");
        assert_eq!(loaded.models[0].name, "Test Model");
        assert_eq!(loaded.models[1].catalog_id, "another-model-q8");
        assert_eq!(loaded.models[1].size_bytes, 2_000_000);
    }

    #[test]
    fn test_registry_load_missing_file() {
        let dir = TempDir::new().unwrap();
        let registry_path = dir.path().join("nonexistent.json");

        let loaded = ModelManager::load_registry(&registry_path).unwrap();
        assert!(loaded.models.is_empty(), "Loading missing file should return empty registry");
    }
}
