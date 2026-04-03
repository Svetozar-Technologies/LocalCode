use crate::storage::hnsw::cosine_similarity;

/// Semantic relevance scoring via vector similarity.
pub fn semantic_relevance(memory_embedding: &[f32], query_embedding: &[f32]) -> f32 {
    cosine_similarity(memory_embedding, query_embedding).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_vectors() {
        let v = vec![1.0, 0.0, 0.5];
        assert!((semantic_relevance(&v, &v) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_orthogonal_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!(semantic_relevance(&a, &b).abs() < 0.01);
    }
}
