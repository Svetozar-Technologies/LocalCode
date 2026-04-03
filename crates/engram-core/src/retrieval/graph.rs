use crate::memory::types::*;
use crate::mesh::graph::CognitiveGraph;

/// Graph-based retrieval using the cognitive memory mesh.
pub struct GraphRetriever<'a> {
    graph: &'a CognitiveGraph,
}

impl<'a> GraphRetriever<'a> {
    pub fn new(graph: &'a CognitiveGraph) -> Self {
        Self { graph }
    }

    /// Score a memory based on its graph proximity to context memories
    pub fn score(&self, memory_id: &MemoryId, context_ids: &[MemoryId], max_hops: usize) -> f32 {
        if context_ids.is_empty() {
            return 0.0;
        }

        let total: f32 = context_ids
            .iter()
            .map(|ctx| self.graph.proximity_score(memory_id, ctx, max_hops))
            .sum();

        total / context_ids.len() as f32
    }

    /// Find memories connected to any of the context memories within max_hops
    pub fn related_memories(&self, context_ids: &[MemoryId], max_hops: usize) -> Vec<(MemoryId, f32)> {
        let mut scores: std::collections::HashMap<MemoryId, f32> = std::collections::HashMap::new();

        for ctx_id in context_ids {
            let reachable = self.graph.bfs(ctx_id, max_hops);
            for (id, depth) in reachable {
                if context_ids.contains(&id) {
                    continue; // Skip context memories themselves
                }
                let score = 1.0 / (1.0 + depth as f32);
                let entry = scores.entry(id).or_insert(0.0);
                *entry = entry.max(score);
            }
        }

        let mut results: Vec<_> = scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }
}
