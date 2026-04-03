use crate::memory::types::*;
use crate::storage::hnsw::{cosine_similarity, HnswIndex};

/// Vector similarity search using the HNSW index.
pub struct VectorRetriever<'a> {
    index: &'a HnswIndex,
}

impl<'a> VectorRetriever<'a> {
    pub fn new(index: &'a HnswIndex) -> Self {
        Self { index }
    }

    /// Find the k most similar memories to the query embedding
    pub fn search(&self, query_embedding: &[f32], k: usize) -> EngramResult<Vec<(MemoryId, f32)>> {
        self.index.search(query_embedding, k)
    }

    /// Score a specific memory against a query embedding
    pub fn score(&self, memory_embedding: &[f32], query_embedding: &[f32]) -> f32 {
        cosine_similarity(memory_embedding, query_embedding).max(0.0)
    }
}
