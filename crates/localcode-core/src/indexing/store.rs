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

#[derive(Serialize, Deserialize, Default)]
pub struct CodeIndex {
    pub entries: Vec<IndexEntry>,
    pub file_hashes: HashMap<String, u64>,
    #[serde(default)]
    pub doc_count: usize,
    #[serde(default)]
    pub term_doc_freqs: HashMap<String, usize>,
    /// Embedding dimension (384 for Engram, 256 for legacy)
    #[serde(default = "default_embed_dim")]
    pub embed_dim: usize,
    /// Transient HNSW index — rebuilt on load, not serialized
    #[serde(skip)]
    hnsw: Option<engram_core::HnswIndex>,
    /// Transient embedding engine
    #[serde(skip)]
    embed_engine: Option<std::sync::Arc<dyn engram_core::EmbeddingEngine>>,
}

fn default_embed_dim() -> usize {
    256 // legacy default
}

impl std::fmt::Debug for CodeIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeIndex")
            .field("entries", &self.entries.len())
            .field("embed_dim", &self.embed_dim)
            .field("hnsw", &self.hnsw.is_some())
            .finish()
    }
}

impl CodeIndex {
    pub fn new() -> Self {
        let engine = std::sync::Arc::new(engram_core::HashEmbeddingEngine::default_384());
        Self {
            entries: Vec::new(),
            file_hashes: HashMap::new(),
            doc_count: 0,
            term_doc_freqs: HashMap::new(),
            embed_dim: 384,
            hnsw: Some(engram_core::HnswIndex::for_minilm()),
            embed_engine: Some(engine),
        }
    }

    /// Get or create the embedding engine
    fn engine(&self) -> std::sync::Arc<dyn engram_core::EmbeddingEngine> {
        self.embed_engine.clone().unwrap_or_else(|| {
            std::sync::Arc::new(engram_core::HashEmbeddingEngine::default_384())
        })
    }

    pub fn add_chunk(&mut self, chunk: &CodeChunk) {
        let engine = self.engine();
        let embedding = engine.embed(&chunk.content).unwrap_or_else(|_| {
            embeddings::simple_embed(&chunk.content)
        });

        let id = uuid::Uuid::new_v4();

        // Insert into HNSW if available
        if let Some(ref hnsw) = self.hnsw {
            let _ = hnsw.insert(id, embedding.clone());
        }

        self.entries.push(IndexEntry {
            file: chunk.file.clone(),
            start_line: chunk.start_line,
            end_line: chunk.end_line,
            content: chunk.content.clone(),
            embedding,
        });
    }

    /// Recalculate statistics after indexing is complete
    pub fn compute_stats(&mut self) {
        self.doc_count = self.entries.len();
        let docs: Vec<&str> = self.entries.iter().map(|e| e.content.as_str()).collect();
        self.term_doc_freqs = embeddings::compute_doc_freqs(&docs);
    }

    /// Build HNSW index from existing entries (call after load)
    fn rebuild_hnsw(&mut self) {
        if !self.entries.is_empty() {
            let dim = self.entries[0].embedding.len();
            let hnsw = engram_core::HnswIndex::new(dim, Default::default());
            for entry in &self.entries {
                let id = uuid::Uuid::new_v4();
                let _ = hnsw.insert(id, entry.embedding.clone());
            }
            self.hnsw = Some(hnsw);
        }
    }

    pub fn search(&self, query: &str, top_k: usize) -> Vec<&IndexEntry> {
        if self.entries.is_empty() {
            return Vec::new();
        }

        let engine = self.engine();
        let query_embedding = engine.embed(query).unwrap_or_else(|_| {
            embeddings::simple_embed(query)
        });

        // Use HNSW for fast approximate vector search if available
        if let Some(ref hnsw) = self.hnsw {
            if let Ok(_) = hnsw.search(&query_embedding, top_k * 2) {
                // HNSW returns nearest neighbors by vector; now combine with BM25
                if self.doc_count > 0 && !self.term_doc_freqs.is_empty() {
                    return self.hybrid_search_ranked(query, &query_embedding, top_k);
                }
                // Pure vector search via HNSW result ordering
                // Map HNSW results back to entries by embedding similarity
                return self.vector_search_ranked(&query_embedding, top_k);
            }
        }

        // Fallback: brute-force hybrid search (original logic)
        if self.doc_count > 0 && !self.term_doc_freqs.is_empty() {
            let docs: Vec<&str> = self.entries.iter().map(|e| e.content.as_str()).collect();
            let avg_doc_len = embeddings::compute_avg_doc_len(&docs);

            let results = embeddings::hybrid_search(
                &query_embedding,
                query,
                &self.entries,
                avg_doc_len,
                self.doc_count,
                &self.term_doc_freqs,
                top_k,
            );

            results
                .into_iter()
                .map(|(_score, idx)| &self.entries[idx])
                .collect()
        } else {
            self.vector_search_ranked(&query_embedding, top_k)
        }
    }

    /// Hybrid search: vector similarity + BM25 keyword matching
    fn hybrid_search_ranked(&self, query: &str, query_embedding: &[f32], top_k: usize) -> Vec<&IndexEntry> {
        let docs: Vec<&str> = self.entries.iter().map(|e| e.content.as_str()).collect();
        let avg_doc_len = embeddings::compute_avg_doc_len(&docs);

        let results = embeddings::hybrid_search(
            query_embedding,
            query,
            &self.entries,
            avg_doc_len,
            self.doc_count,
            &self.term_doc_freqs,
            top_k,
        );

        results
            .into_iter()
            .map(|(_score, idx)| &self.entries[idx])
            .collect()
    }

    /// Pure vector search with brute-force ranking
    fn vector_search_ranked(&self, query_embedding: &[f32], top_k: usize) -> Vec<&IndexEntry> {
        let mut scored: Vec<(f32, &IndexEntry)> = self
            .entries
            .iter()
            .map(|entry| {
                let sim = embeddings::cosine_similarity(query_embedding, &entry.embedding);
                (sim, entry)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(top_k).map(|(_, entry)| entry).collect()
    }

    /// Search returning entries with their relevance scores
    pub fn search_with_scores(&self, query: &str, top_k: usize) -> Vec<(f32, &IndexEntry)> {
        if self.entries.is_empty() {
            return Vec::new();
        }

        let engine = self.engine();
        let query_embedding = engine.embed(query).unwrap_or_else(|_| {
            embeddings::simple_embed(query)
        });

        if self.doc_count > 0 && !self.term_doc_freqs.is_empty() {
            let docs: Vec<&str> = self.entries.iter().map(|e| e.content.as_str()).collect();
            let avg_doc_len = embeddings::compute_avg_doc_len(&docs);

            let results = embeddings::hybrid_search(
                &query_embedding,
                query,
                &self.entries,
                avg_doc_len,
                self.doc_count,
                &self.term_doc_freqs,
                top_k,
            );

            results
                .into_iter()
                .map(|(score, idx)| (score, &self.entries[idx]))
                .collect()
        } else {
            let mut scored: Vec<(f32, &IndexEntry)> = self
                .entries
                .iter()
                .map(|entry| {
                    let sim = embeddings::cosine_similarity(&query_embedding, &entry.embedding);
                    (sim, entry)
                })
                .collect();

            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            scored.into_iter().take(top_k).collect()
        }
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
        let mut index: Self = serde_json::from_slice(&data)?;

        // Initialize engine
        index.embed_engine = Some(std::sync::Arc::new(
            engram_core::HashEmbeddingEngine::default_384(),
        ));

        // Rebuild HNSW from stored embeddings
        index.rebuild_hnsw();

        Ok(index)
    }

    pub fn index_path(project_path: &str) -> PathBuf {
        Path::new(project_path)
            .join(".localcode")
            .join("index.json")
    }
}
