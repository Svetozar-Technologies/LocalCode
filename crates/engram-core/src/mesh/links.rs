//! Cross-memory-type edge creation utilities.
//!
//! Provides helper functions for creating semantically meaningful
//! links between memories of different types.

use crate::memory::types::*;
use crate::mesh::graph::CognitiveGraph;

/// Link an episodic memory to a semantic fact it supports
pub fn link_evidence(graph: &CognitiveGraph, episodic: MemoryId, semantic: MemoryId) {
    graph.add_link(episodic, semantic, LinkType::Refines, 0.8);
}

/// Link two memories as temporally ordered (A happened before B)
pub fn link_temporal_order(graph: &CognitiveGraph, earlier: MemoryId, later: MemoryId) {
    graph.add_link(earlier, later, LinkType::Temporal, 1.0);
}

/// Mark two memories as contradicting each other
pub fn link_contradiction(graph: &CognitiveGraph, a: MemoryId, b: MemoryId) {
    graph.add_link(a, b, LinkType::Contradicts, 1.0);
    graph.add_link(b, a, LinkType::Contradicts, 1.0);
}

/// Link a consolidated semantic memory to its source episodic memories
pub fn link_consolidation(graph: &CognitiveGraph, sources: &[MemoryId], consolidated: MemoryId) {
    for &source in sources {
        graph.add_link(consolidated, source, LinkType::DerivedFrom, 1.0);
    }
}

/// Create a semantic similarity link between two memories
pub fn link_semantic_similarity(graph: &CognitiveGraph, a: MemoryId, b: MemoryId, similarity: f32) {
    graph.add_link(a, b, LinkType::Semantic, similarity);
    graph.add_link(b, a, LinkType::Semantic, similarity);
}

/// Create a cross-agent link (e.g., when sharing memories)
pub fn link_cross_agent(graph: &CognitiveGraph, source: MemoryId, shared: MemoryId) {
    graph.add_link(source, shared, LinkType::CrossAgent, 1.0);
}

/// Create a causal chain: A -> B -> C
pub fn link_causal_chain(graph: &CognitiveGraph, chain: &[MemoryId]) {
    for window in chain.windows(2) {
        graph.add_link(window[0], window[1], LinkType::Causal, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_causal_chain() {
        let graph = CognitiveGraph::new();
        let ids: Vec<_> = (0..4).map(|_| Uuid::now_v7()).collect();

        link_causal_chain(&graph, &ids);

        assert_eq!(graph.edge_count(), 3);

        let neighbors = graph.neighbors_by_type(&ids[0], &LinkType::Causal);
        assert_eq!(neighbors.len(), 1);
    }

    #[test]
    fn test_contradiction_bidirectional() {
        let graph = CognitiveGraph::new();
        let a = Uuid::now_v7();
        let b = Uuid::now_v7();

        link_contradiction(&graph, a, b);

        assert!(!graph.find_contradictions(&a).is_empty());
        assert!(!graph.find_contradictions(&b).is_empty());
    }
}
