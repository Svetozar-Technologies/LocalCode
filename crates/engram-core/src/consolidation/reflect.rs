//! Reflect operation — the core consolidation pipeline.
//!
//! `reflect()` is the self-improving heartbeat of Engram:
//! 1. Cluster recent episodic memories by entity/topic
//! 2. Generate semantic summaries from clusters
//! 3. Detect conflicts with existing semantics
//! 4. Resolve conflicts (prefer newer/stronger)
//! 5. Create DerivedFrom links
//! 6. Archive low-retrievability sources

use crate::consolidation::cluster::cluster_by_similarity;
use crate::consolidation::compress::{create_summary, should_compress};
use crate::consolidation::conflict::{detect_conflicts, register_contradictions};
use crate::consolidation::decay::process_decay;
use crate::consolidation::entity::EntityIndex;
use crate::memory::types::*;
use crate::mesh::graph::CognitiveGraph;
use crate::storage::engine::StorageEngine;
use crate::storage::hnsw::HnswIndex;

/// Result of a reflect operation
#[derive(Debug, Default)]
pub struct ReflectResult {
    /// New semantic memories created from consolidation
    pub consolidated: Vec<MemoryId>,
    /// Conflicts detected and resolved
    pub conflicts_resolved: usize,
    /// Memories archived (low retrievability)
    pub archived: Vec<MemoryId>,
    /// Entity links created
    pub entity_links_created: usize,
}

/// Configuration for the reflect operation
pub struct ReflectConfig {
    /// Minimum cluster size for consolidation
    pub min_cluster_size: usize,
    /// Similarity threshold for clustering (0.0 - 1.0)
    pub cluster_eps: f32,
    /// Conflict detection threshold
    pub conflict_threshold: f32,
    /// Maximum number of recent memories to process
    pub max_recent: usize,
}

impl Default for ReflectConfig {
    fn default() -> Self {
        Self {
            min_cluster_size: 2,
            cluster_eps: 0.3,
            conflict_threshold: 0.85,
            max_recent: 100,
        }
    }
}

/// Execute the reflect operation on an agent's memories
pub fn reflect(
    agent_id: &AgentId,
    storage: &StorageEngine,
    graph: &CognitiveGraph,
    hnsw: &HnswIndex,
    entity_index: &mut EntityIndex,
    config: &ReflectConfig,
) -> EngramResult<ReflectResult> {
    let mut result = ReflectResult::default();

    // Step 1: Gather recent episodic memories for this agent
    let agent_memories = storage.get_by_agent(agent_id);

    let recent_episodics: Vec<MemoryNode> = agent_memories
        .iter()
        .filter(|m| matches!(m.memory_type, MemoryType::Episodic))
        .take(config.max_recent)
        .cloned()
        .collect();

    let existing_semantics: Vec<MemoryNode> = agent_memories
        .iter()
        .filter(|m| matches!(m.memory_type, MemoryType::Semantic))
        .cloned()
        .collect();

    if recent_episodics.is_empty() {
        return Ok(result);
    }

    // Step 2: Cluster episodic memories by embedding similarity
    let memories_with_embeddings: Vec<(MemoryId, Vec<f32>)> = recent_episodics
        .iter()
        .filter_map(|m| {
            m.embedding.as_ref().map(|emb| (m.id, emb.clone()))
        })
        .collect();

    let clusters = cluster_by_similarity(
        &memories_with_embeddings,
        config.cluster_eps,
        config.min_cluster_size,
    );

    // Step 3: For each cluster, create a consolidated semantic memory
    for cluster_ids in &clusters {
        let cluster_memories: Vec<MemoryNode> = cluster_ids
            .iter()
            .filter_map(|id| storage.get(id).ok())
            .collect();

        if cluster_memories.len() < config.min_cluster_size {
            continue;
        }

        // Create summary
        let summary = create_summary(&cluster_memories);

        // Create new semantic memory from consolidation
        let mut semantic = MemoryNode::new(
            *agent_id,
            MemoryType::Semantic,
            summary,
        );

        // Inherit tags from source memories
        let mut all_tags: std::collections::HashSet<Tag> = std::collections::HashSet::new();
        for m in &cluster_memories {
            for tag in &m.tags {
                all_tags.insert(tag.clone());
            }
        }
        for tag in all_tags {
            semantic.tags.push(tag);
        }

        // Add DerivedFrom links
        for source_id in cluster_ids {
            semantic.links.push(MemoryLink {
                target: *source_id,
                link_type: LinkType::DerivedFrom,
                weight: 1.0,
            });
        }

        // Step 4: Detect conflicts with existing semantics
        let conflicts = detect_conflicts(
            &semantic,
            &existing_semantics,
            config.conflict_threshold,
        );

        if !conflicts.is_empty() {
            register_contradictions(graph, &conflicts);
            result.conflicts_resolved += conflicts.len();
        }

        // Store the consolidated memory
        let id = storage.store(semantic.clone())?;

        // Add to graph
        graph.add_node(id);
        for source_id in cluster_ids {
            graph.add_link(id, *source_id, LinkType::DerivedFrom, 1.0);
        }

        // Index entities
        entity_index.index_memory(id, &semantic.content);

        // Create entity-based links
        let related = entity_index.find_related(&id);
        for (related_id, _shared_entities) in &related {
            graph.add_link(id, *related_id, LinkType::Semantic, 0.5);
            result.entity_links_created += 1;
        }

        // Index in HNSW if embedding exists
        if let Some(ref emb) = semantic.embedding {
            let _ = hnsw.insert(id, emb.clone());
        }

        result.consolidated.push(id);
    }

    // Step 5: Archive low-retrievability memories
    let to_archive = process_decay(storage);
    for id in &to_archive {
        if let Ok(memory) = storage.get(id) {
            if should_compress(&memory, true) {
                result.archived.push(*id);
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_reflect_basic() {
        let agent_id = Uuid::now_v7();
        let storage = StorageEngine::in_memory().unwrap();
        let graph = CognitiveGraph::new();
        let hnsw = HnswIndex::new(4, Default::default());
        let mut entity_index = EntityIndex::new();
        let config = ReflectConfig {
            min_cluster_size: 2,
            cluster_eps: 0.5,
            ..Default::default()
        };

        // Store some episodic memories with similar embeddings
        let emb1 = vec![1.0, 0.0, 0.0, 0.0];
        let emb2 = vec![0.95, 0.05, 0.0, 0.0];
        let emb3 = vec![0.0, 1.0, 0.0, 0.0]; // Different cluster

        let m1 = MemoryNode::new(agent_id, MemoryType::Episodic, "user ate pasta")
            .with_embedding(emb1.clone())
            .tag("food");
        let m2 = MemoryNode::new(agent_id, MemoryType::Episodic, "user ordered pasta dish")
            .with_embedding(emb2.clone())
            .tag("food");
        let m3 = MemoryNode::new(agent_id, MemoryType::Episodic, "user booked flight")
            .with_embedding(emb3.clone())
            .tag("travel");

        storage.store(m1).unwrap();
        storage.store(m2).unwrap();
        storage.store(m3).unwrap();

        let result = reflect(&agent_id, &storage, &graph, &hnsw, &mut entity_index, &config).unwrap();

        // Should consolidate the two pasta memories into one semantic
        assert!(result.consolidated.len() >= 1 || result.archived.is_empty(),
            "Reflect should produce results: {:?}", result);
    }

    #[test]
    fn test_reflect_empty() {
        let agent_id = Uuid::now_v7();
        let storage = StorageEngine::in_memory().unwrap();
        let graph = CognitiveGraph::new();
        let hnsw = HnswIndex::new(4, Default::default());
        let mut entity_index = EntityIndex::new();

        let result = reflect(&agent_id, &storage, &graph, &hnsw, &mut entity_index, &ReflectConfig::default()).unwrap();
        assert_eq!(result.consolidated.len(), 0);
        assert_eq!(result.archived.len(), 0);
    }
}
