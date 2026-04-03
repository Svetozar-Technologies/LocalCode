use crate::memory::types::*;
use crate::mesh::graph::CognitiveGraph;
use crate::retrieval::graph::GraphRetriever;
use crate::retrieval::keyword::{keyword_overlap_score, KeywordIndex};
use crate::retrieval::temporal::combined_temporal_score;
use crate::storage::engine::StorageEngine;
use crate::storage::hnsw::{cosine_similarity, HnswIndex};
use chrono::Utc;

/// Reranking strategy for hybrid retrieval
#[derive(Clone, Debug, Default)]
pub enum RerankStrategy {
    /// Linear weighted sum of scores (default)
    #[default]
    WeightedSum,
    /// Reciprocal Rank Fusion — combines rank positions across retrievers
    RRF { k: usize },
    /// Maximal Marginal Relevance — balances relevance and diversity
    MMR { lambda: f32 },
}

/// Hybrid retrieval engine that fuses multiple retrieval signals.
///
/// Combines:
/// - Vector similarity (semantic match)
/// - Temporal relevance (recency)
/// - Memory strength (retrievability)
/// - Graph proximity (relational context)
/// - Keyword match (lexical overlap)
///
/// Each signal is weighted according to the query's ScoringWeights.
pub struct HybridRetriever<'a> {
    storage: &'a StorageEngine,
    hnsw: &'a HnswIndex,
    graph: &'a CognitiveGraph,
    keyword_index: &'a KeywordIndex,
}

impl<'a> HybridRetriever<'a> {
    pub fn new(
        storage: &'a StorageEngine,
        hnsw: &'a HnswIndex,
        graph: &'a CognitiveGraph,
        keyword_index: &'a KeywordIndex,
    ) -> Self {
        Self {
            storage,
            hnsw,
            graph,
            keyword_index,
        }
    }

    /// Execute a hybrid recall query
    pub fn recall(&self, query: &RecallQuery, rerank: &RerankStrategy) -> Vec<ScoredMemory> {
        let now = Utc::now();
        let weights = &query.weights;

        // Phase 1: Gather candidates from multiple sources
        let mut candidate_ids: std::collections::HashSet<MemoryId> = std::collections::HashSet::new();

        // Vector search candidates (with rank tracking for RRF)
        let mut vector_ranks: std::collections::HashMap<MemoryId, usize> = std::collections::HashMap::new();
        if let Some(ref embedding) = query.embedding {
            if let Ok(vector_results) = self.hnsw.search(embedding, query.limit * 3) {
                for (rank, (id, _)) in vector_results.iter().enumerate() {
                    candidate_ids.insert(*id);
                    vector_ranks.insert(*id, rank + 1);
                }
            }
        }

        // Keyword search candidates (with rank tracking for RRF)
        let mut keyword_ranks: std::collections::HashMap<MemoryId, usize> = std::collections::HashMap::new();
        if let Some(ref text) = query.text {
            let kw_results = self.keyword_index.search(text, query.limit * 3);
            for (rank, (id, _)) in kw_results.iter().enumerate() {
                candidate_ids.insert(*id);
                keyword_ranks.insert(*id, rank + 1);
            }
        }

        // If no embedding or text, use all memories for the agent
        if candidate_ids.is_empty() {
            if let Some(agent_id) = query.agent_id {
                let agent_mems = self.storage.get_by_agent(&agent_id);
                for m in &agent_mems {
                    candidate_ids.insert(m.id);
                }
            } else {
                for id in self.storage.all_ids().into_iter().take(query.limit * 10) {
                    candidate_ids.insert(id);
                }
            }
        }

        // Phase 2: Score each candidate
        let mut scored: Vec<ScoredMemory> = candidate_ids
            .into_iter()
            .filter_map(|id| {
                let memory = self.storage.get(&id).ok()?;

                // Apply filters
                if !self.passes_filters(&memory, query) {
                    return None;
                }

                let breakdown = self.score_memory(&memory, query, now);

                let score = match rerank {
                    RerankStrategy::WeightedSum => {
                        breakdown.vector_score * weights.vector
                            + breakdown.temporal_score * weights.temporal
                            + breakdown.strength_score * weights.strength
                            + breakdown.graph_score * weights.graph
                            + breakdown.keyword_score * weights.keyword
                    }
                    RerankStrategy::RRF { k } => {
                        // Reciprocal Rank Fusion: sum of 1/(k + rank) across retrievers
                        let k = *k as f32;
                        let vr = vector_ranks.get(&id).map(|r| 1.0 / (k + *r as f32)).unwrap_or(0.0);
                        let kr = keyword_ranks.get(&id).map(|r| 1.0 / (k + *r as f32)).unwrap_or(0.0);
                        // Include temporal and strength as bonus signals
                        vr + kr + breakdown.temporal_score * 0.1 + breakdown.strength_score * 0.1 + breakdown.graph_score * 0.1
                    }
                    RerankStrategy::MMR { .. } => {
                        // Initial score for MMR — will be reranked in Phase 3
                        breakdown.vector_score * weights.vector
                            + breakdown.temporal_score * weights.temporal
                            + breakdown.strength_score * weights.strength
                            + breakdown.graph_score * weights.graph
                            + breakdown.keyword_score * weights.keyword
                    }
                };

                Some(ScoredMemory {
                    memory,
                    score,
                    score_breakdown: breakdown,
                })
            })
            .collect();

        // Phase 3: Rerank
        match rerank {
            RerankStrategy::MMR { lambda } => {
                scored = self.mmr_rerank(scored, query, *lambda, query.limit);
            }
            _ => {
                scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
                scored.truncate(query.limit);
            }
        }

        // Phase 4: Record access for returned memories
        for result in &scored {
            let _ = self.storage.record_access(&result.memory.id);
        }

        scored
    }

    /// Maximal Marginal Relevance: greedily select results that are
    /// both relevant to query AND diverse from already-selected results
    fn mmr_rerank(
        &self,
        candidates: Vec<ScoredMemory>,
        _query: &RecallQuery,
        lambda: f32,
        limit: usize,
    ) -> Vec<ScoredMemory> {
        if candidates.is_empty() {
            return candidates;
        }

        let mut selected: Vec<ScoredMemory> = Vec::with_capacity(limit);
        let mut remaining: Vec<ScoredMemory> = candidates;

        for _ in 0..limit {
            if remaining.is_empty() {
                break;
            }

            let best_idx = remaining
                .iter()
                .enumerate()
                .map(|(i, candidate)| {
                    let relevance = candidate.score;

                    // Max similarity to any already-selected result
                    let max_sim = selected
                        .iter()
                        .map(|s| {
                            match (&candidate.memory.embedding, &s.memory.embedding) {
                                (Some(a), Some(b)) => cosine_similarity(a, b),
                                _ => 0.0,
                            }
                        })
                        .fold(0.0f32, f32::max);

                    let mmr_score = lambda * relevance - (1.0 - lambda) * max_sim;
                    (i, mmr_score)
                })
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i)
                .unwrap_or(0);

            selected.push(remaining.remove(best_idx));
        }

        selected
    }

    fn score_memory(
        &self,
        memory: &MemoryNode,
        query: &RecallQuery,
        now: chrono::DateTime<Utc>,
    ) -> ScoreBreakdown {
        // Vector similarity
        let vector_score = match (&memory.embedding, &query.embedding) {
            (Some(mem_emb), Some(q_emb)) => {
                crate::storage::hnsw::cosine_similarity(mem_emb, q_emb).max(0.0)
            }
            _ => 0.0,
        };

        // Temporal relevance
        let temporal_score = combined_temporal_score(&memory.temporal, now);

        // Memory strength (retrievability)
        let strength_score = memory.strength.retrievability_at(now);

        // Graph proximity (if context is available)
        let graph_score = match &query.context_ids {
            Some(ctx_ids) if !ctx_ids.is_empty() => {
                let graph_retriever = GraphRetriever::new(self.graph);
                graph_retriever.score(&memory.id, ctx_ids, 3)
            }
            _ => 0.0,
        };

        // Keyword match
        let keyword_score = match &query.text {
            Some(text) => keyword_overlap_score(&memory.content, text),
            None => 0.0,
        };

        ScoreBreakdown {
            vector_score,
            temporal_score,
            strength_score,
            graph_score,
            keyword_score,
        }
    }

    fn passes_filters(&self, memory: &MemoryNode, query: &RecallQuery) -> bool {
        // Agent filter
        if let Some(agent_id) = &query.agent_id {
            if memory.agent_id != *agent_id {
                // Check visibility for cross-agent access
                if !memory.visibility.can_access(agent_id, &memory.agent_id) {
                    return false;
                }
            }
        }

        // Memory type filter
        if let Some(ref types) = query.memory_types {
            if !types.iter().any(|t| t.matches(&memory.memory_type)) {
                return false;
            }
        }

        // Tag filter
        if let Some(ref tags) = query.tags {
            if !tags.iter().any(|t| memory.tags.iter().any(|mt| mt == t)) {
                return false;
            }
        }

        // Time range filter
        if let Some(ref range) = query.time_range {
            if let Some(start) = range.start {
                if memory.temporal.created_at < start {
                    return false;
                }
            }
            if let Some(end) = range.end {
                if memory.temporal.created_at > end {
                    return false;
                }
            }
        }

        // Strength filter
        if let Some(min) = query.min_strength {
            if memory.retrievability() < min {
                return false;
            }
        }

        // Bi-temporal validity filter
        if let Some(as_of) = query.as_of_time {
            if !memory.temporal.is_valid_at(as_of) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_hybrid_recall_basic() {
        let storage = StorageEngine::in_memory().unwrap();
        let hnsw = HnswIndex::new(4, Default::default());
        let graph = CognitiveGraph::new();
        let mut kw_index = KeywordIndex::new();

        let agent = Uuid::now_v7();

        // Store some memories
        let node1 = MemoryNode::new(agent, MemoryType::Semantic, "user is vegetarian")
            .tag("diet");
        let node2 = MemoryNode::new(agent, MemoryType::Semantic, "user likes pasta");
        let node3 = MemoryNode::new(agent, MemoryType::Episodic, "booked flight to paris");

        for node in [&node1, &node2, &node3] {
            storage.store(node.clone()).unwrap();
            kw_index.add(node.id, &node.content);
        }

        let retriever = HybridRetriever::new(&storage, &hnsw, &graph, &kw_index);

        let query = RecallQuery::new("vegetarian diet preferences")
            .with_agent(agent)
            .with_limit(10);

        let results = retriever.recall(&query, &RerankStrategy::default());
        assert!(!results.is_empty());

        // "vegetarian" memory should rank highly
        assert!(results[0].memory.content.as_str().contains("vegetarian"));
    }

    #[test]
    fn test_hybrid_recall_with_type_filter() {
        let storage = StorageEngine::in_memory().unwrap();
        let hnsw = HnswIndex::new(4, Default::default());
        let graph = CognitiveGraph::new();
        let mut kw_index = KeywordIndex::new();

        let agent = Uuid::now_v7();

        let node1 = MemoryNode::new(agent, MemoryType::Semantic, "vegetarian fact");
        let node2 = MemoryNode::new(agent, MemoryType::Episodic, "vegetarian meal ordered");

        for node in [&node1, &node2] {
            storage.store(node.clone()).unwrap();
            kw_index.add(node.id, &node.content);
        }

        let retriever = HybridRetriever::new(&storage, &hnsw, &graph, &kw_index);

        let query = RecallQuery::new("vegetarian")
            .with_agent(agent)
            .with_types(vec![MemoryTypeFilter::Semantic])
            .with_limit(10);

        let results = retriever.recall(&query, &RerankStrategy::default());
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].memory.memory_type, MemoryType::Semantic));
    }
}
