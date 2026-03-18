use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::chunker::CodeChunk;
use super::embeddings;
use crate::CoreResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CodeIndex {
    pub entries: Vec<IndexEntry>,
    pub file_hashes: HashMap<String, u64>,
}

impl CodeIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_chunk(&mut self, chunk: &CodeChunk) {
        let embedding = embeddings::simple_embed(&chunk.content);
        self.entries.push(IndexEntry {
            file: chunk.file.clone(),
            start_line: chunk.start_line,
            end_line: chunk.end_line,
            content: chunk.content.clone(),
            embedding,
        });
    }

    pub fn search(&self, query: &str, top_k: usize) -> Vec<&IndexEntry> {
        let query_embedding = embeddings::simple_embed(query);

        let mut scored: Vec<(f32, &IndexEntry)> = self
            .entries
            .iter()
            .map(|entry| {
                let sim = embeddings::cosine_similarity(&query_embedding, &entry.embedding);
                (sim, entry)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(top_k).map(|(_, entry)| entry).collect()
    }

    pub fn remove_file(&mut self, file: &str) {
        self.entries.retain(|e| e.file != file);
        self.file_hashes.remove(file);
    }

    pub fn save(&self, path: &Path) -> CoreResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_vec(self)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    pub fn load(path: &Path) -> CoreResult<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let data = std::fs::read(path)?;
        let index: Self = serde_json::from_slice(&data)?;
        Ok(index)
    }

    pub fn index_path(project_path: &str) -> PathBuf {
        Path::new(project_path)
            .join(".localcode")
            .join("index.json")
    }
}
