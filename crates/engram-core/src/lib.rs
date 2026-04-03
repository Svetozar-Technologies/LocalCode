//! Engram: A novel local memory system for AI agents.
//!
//! Combines cognitive science-inspired memory types (episodic, semantic,
//! procedural, prospective, working) with a graph-based memory mesh,
//! hybrid retrieval, and biology-inspired cross-agent sharing.

pub mod memory;
pub mod mesh;
pub mod storage;
pub mod retrieval;
pub mod consolidation;
pub mod membrane;
pub mod embedding;
pub mod scoring;

// Re-export core types for convenience
pub use memory::types::*;
pub use storage::engine::StorageEngine;
pub use mesh::graph::CognitiveGraph;
pub use embedding::engine::{EmbeddingEngine, HashEmbeddingEngine, OnnxEmbeddingEngine};
pub use storage::hnsw::HnswIndex;
pub use retrieval::keyword::KeywordIndex;
pub use retrieval::hybrid::{HybridRetriever, RerankStrategy};
pub use membrane::namespace::NamespaceManager;
pub use membrane::permeability::PermeabilityController;
pub use membrane::audit::AuditLog;

use crate::consolidation::entity::EntityIndex;
use crate::consolidation::reflect::{self, ReflectConfig, ReflectResult};
use crate::memory::prospective;
use crate::memory::working::WorkingMemoryBuffer;

use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The main Engram runtime, composing all subsystems.
pub struct Engram {
    pub storage: StorageEngine,
    pub graph: CognitiveGraph,
    pub hnsw: HnswIndex,
    pub keyword_index: parking_lot::RwLock<KeywordIndex>,
    pub namespaces: NamespaceManager,
    pub audit: AuditLog,
    pub embedding_engine: Arc<dyn EmbeddingEngine>,
    /// Working memory buffers per agent
    pub working_buffers: dashmap::DashMap<AgentId, WorkingMemoryBuffer>,
    /// Entity index for cross-memory entity resolution
    pub entity_index: parking_lot::RwLock<EntityIndex>,
    data_dir: PathBuf,
}

impl Engram {
    /// Create a new Engram instance with persistent storage
    pub fn open(data_dir: impl AsRef<Path>) -> EngramResult<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        let storage = StorageEngine::open(&data_dir)?;
        let graph = CognitiveGraph::new();
        let hnsw = HnswIndex::for_minilm();
        let keyword_index = parking_lot::RwLock::new(KeywordIndex::new());
        let namespaces = NamespaceManager::new();
        let audit = AuditLog::default();
        // Try real ONNX embeddings first, fall back to hash-based
        let embedding_engine: Arc<dyn EmbeddingEngine> = match OnnxEmbeddingEngine::try_default() {
            Ok(onnx) => {
                tracing::info!("Using ONNX embedding engine (all-MiniLM-L6-v2, 384-dim)");
                Arc::new(onnx)
            }
            Err(_) => {
                tracing::debug!("ONNX embeddings not available, using hash fallback");
                Arc::new(HashEmbeddingEngine::default_384())
            }
        };

        // Rebuild indices from storage
        let engram = Self {
            storage,
            graph,
            hnsw,
            keyword_index,
            namespaces,
            audit,
            embedding_engine,
            working_buffers: dashmap::DashMap::new(),
            entity_index: parking_lot::RwLock::new(EntityIndex::new()),
            data_dir,
        };
        engram.rebuild_indices();

        Ok(engram)
    }

    /// Create an in-memory Engram instance (for testing)
    pub fn in_memory() -> EngramResult<Self> {
        let dir = std::env::temp_dir().join(format!("engram-{}", uuid::Uuid::now_v7()));
        Self::open(dir)
    }

    /// Register a new agent
    pub fn register_agent(&self, name: impl Into<String>) -> AgentNamespace {
        self.namespaces.register(name)
    }

    /// Store a memory with automatic embedding and indexing
    pub fn store(&self, mut node: MemoryNode) -> EngramResult<MemoryId> {
        // Generate embedding if not present
        if node.embedding.is_none() {
            match self.embedding_engine.embed(&node.content) {
                Ok(emb) => node.embedding = Some(emb),
                Err(e) => tracing::warn!("Failed to generate embedding: {}", e),
            }
        }

        let id = node.id;

        // Index in HNSW
        if let Some(ref emb) = node.embedding {
            self.hnsw.insert(id, emb.clone())?;
        }

        // Index keywords
        self.keyword_index.write().add(id, &node.content);

        // Add to graph
        self.graph.add_node(id);
        for link in &node.links {
            self.graph.add_link(id, link.target, link.link_type.clone(), link.weight);
        }

        // Update namespace stats
        self.namespaces.increment_memory_count(&node.agent_id);

        // Store in engine (WAL + in-memory)
        self.storage.store(node)?;

        Ok(id)
    }

    /// Store a batch of memories atomically
    pub fn store_batch(&self, nodes: Vec<MemoryNode>) -> EngramResult<Vec<MemoryId>> {
        let mut ids = Vec::with_capacity(nodes.len());
        for node in nodes {
            ids.push(self.store(node)?);
        }
        Ok(ids)
    }

    /// Execute multiple recall queries, returning results for each
    pub fn recall_batch(&self, queries: Vec<RecallQuery>) -> Vec<Vec<ScoredMemory>> {
        queries.into_iter().map(|q| self.recall(q)).collect()
    }

    /// Recall memories using hybrid retrieval
    pub fn recall(&self, query: RecallQuery) -> Vec<ScoredMemory> {
        let mut query = query;

        // Generate query embedding if text provided but no embedding
        if query.embedding.is_none() {
            if let Some(ref text) = query.text {
                match self.embedding_engine.embed(text) {
                    Ok(emb) => query.embedding = Some(emb),
                    Err(e) => tracing::warn!("Failed to embed query: {}", e),
                }
            }
        }

        let kw_index = self.keyword_index.read();
        let retriever = HybridRetriever::new(&self.storage, &self.hnsw, &self.graph, &kw_index);
        retriever.recall(&query, &RerankStrategy::default())
    }

    /// Recall across agent boundaries (respecting permeability)
    pub fn recall_across(
        &self,
        requester: AgentId,
        target_agent: AgentId,
        query: RecallQuery,
    ) -> Vec<ScoredMemory> {
        let ctrl = PermeabilityController::new(&self.namespaces);
        let mut q = query;
        q.agent_id = Some(target_agent);

        let results = self.recall(q);

        // Filter by permeability
        results
            .into_iter()
            .filter(|scored| {
                let granted = ctrl.can_access(&requester, &scored.memory);
                self.audit.record(
                    requester,
                    scored.memory.agent_id,
                    scored.memory.id,
                    membrane::audit::AuditAction::Query,
                    granted,
                );
                granted
            })
            .collect()
    }

    /// Forget (delete) a memory
    pub fn forget(&self, id: &MemoryId) -> EngramResult<()> {
        // Get agent_id before deletion for stats
        let agent_id = self.storage.get(id)?.agent_id;

        self.hnsw.remove(id);
        self.keyword_index.write().remove(id);
        self.graph.remove_node(id);
        self.namespaces.decrement_memory_count(&agent_id);
        self.storage.delete(id)
    }

    // ── Working Memory ─────────────────────────────────────────────────

    /// Push content into an agent's working memory buffer
    pub fn push_working(&self, agent_id: AgentId, content: impl Into<compact_str::CompactString>, ttl_seconds: u64) -> MemoryId {
        let mut buf = self.working_buffers
            .entry(agent_id)
            .or_insert_with(|| WorkingMemoryBuffer::new(agent_id, 50));
        buf.push(content, ttl_seconds)
    }

    /// Get active working memory context for an agent
    pub fn working_context(&self, agent_id: &AgentId) -> Option<String> {
        self.working_buffers.get(agent_id).map(|buf| buf.context_string())
    }

    /// GC expired working memory across all agents
    pub fn gc_working_memory(&self) -> usize {
        let mut total = 0;
        for mut buf in self.working_buffers.iter_mut() {
            total += buf.gc();
        }
        total
    }

    // ── Prospective Memory ──────────────────────────────────────────────

    /// Check all prospective memories for an agent against current context
    pub fn check_triggers(&self, agent_id: &AgentId, context: &str) -> Vec<ScoredMemory> {
        let now = chrono::Utc::now();
        let memories = self.storage.get_by_agent(agent_id);

        memories
            .into_iter()
            .filter(|m| matches!(m.memory_type, MemoryType::Prospective { .. }))
            .filter(|m| prospective::should_trigger(m, context, now))
            .map(|m| ScoredMemory {
                score: 1.0,
                score_breakdown: ScoreBreakdown::default(),
                memory: m,
            })
            .collect()
    }

    // ── Reflect (Consolidation) ─────────────────────────────────────────

    /// Run the reflect consolidation pipeline for an agent
    pub fn reflect(&self, agent_id: &AgentId) -> EngramResult<ReflectResult> {
        let mut entity_index = self.entity_index.write();
        reflect::reflect(
            agent_id,
            &self.storage,
            &self.graph,
            &self.hnsw,
            &mut entity_index,
            &ReflectConfig::default(),
        )
    }

    /// Link two memories
    pub fn link(&self, from: MemoryId, to: MemoryId, link_type: LinkType, weight: f32) -> EngramResult<()> {
        // Verify both exist
        self.storage.get(&from)?;
        self.storage.get(&to)?;

        self.graph.add_link(from, to, link_type, weight);
        Ok(())
    }

    /// Get agent statistics
    pub fn agent_stats(&self, agent_id: &AgentId) -> Option<AgentStats> {
        self.namespaces.get(agent_id).map(|ns| AgentStats {
            agent_id: ns.agent_id,
            display_name: ns.display_name,
            memory_count: self.storage.count_by_agent(agent_id),
            created_at: ns.created_at,
        })
    }

    /// Persist all indices to disk and write a WAL checkpoint.
    /// On next startup, only WAL entries after this checkpoint need replay.
    pub fn snapshot_indices(&self) -> EngramResult<()> {
        let indices_dir = self.data_dir.join("indices");
        std::fs::create_dir_all(&indices_dir)?;

        self.hnsw.save(indices_dir.join("hnsw.bin"))?;
        self.keyword_index.read().save(indices_dir.join("keyword.bin"))?;
        self.graph.save(indices_dir.join("graph.bin"))?;
        self.storage.checkpoint()?;

        tracing::info!("Index snapshot complete");
        Ok(())
    }

    fn rebuild_indices(&self) {
        for memory in self.storage.iter() {
            if let Some(ref emb) = memory.embedding {
                let _ = self.hnsw.insert(memory.id, emb.clone());
            }
            self.keyword_index.write().add(memory.id, &memory.content);
            self.graph.add_node(memory.id);
            for link in &memory.links {
                self.graph.add_link(memory.id, link.target, link.link_type.clone(), link.weight);
            }
        }
    }
}

/// Agent statistics
pub struct AgentStats {
    pub agent_id: AgentId,
    pub display_name: String,
    pub memory_count: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engram_end_to_end() {
        let engram = Engram::in_memory().unwrap();

        // Register agents
        let diet = engram.register_agent("DietAgent");
        let travel = engram.register_agent("TravelAgent");

        // DietAgent stores knowledge
        let fact = MemoryNode::new(diet.agent_id, MemoryType::Semantic, "user is vegetarian")
            .tag("diet")
            .with_visibility(Visibility::Global);

        let _id = engram.store(fact).unwrap();

        // DietAgent can recall its own memories
        let results = engram.recall(
            RecallQuery::new("dietary preferences")
                .with_agent(diet.agent_id)
                .with_limit(5),
        );
        assert!(!results.is_empty());

        // TravelAgent can query across agents (global visibility)
        let cross_results = engram.recall_across(
            travel.agent_id,
            diet.agent_id,
            RecallQuery::new("food preferences").with_limit(5),
        );
        assert!(!cross_results.is_empty());
        assert!(cross_results[0].memory.content.as_str().contains("vegetarian"));
    }

    #[test]
    fn test_engram_permeability() {
        let engram = Engram::in_memory().unwrap();

        let diet = engram.register_agent("DietAgent");
        let travel = engram.register_agent("TravelAgent");

        // Private memory
        let private_fact = MemoryNode::new(diet.agent_id, MemoryType::Semantic, "secret diet info");
        engram.store(private_fact).unwrap();

        // TravelAgent shouldn't see private memories
        let results = engram.recall_across(
            travel.agent_id,
            diet.agent_id,
            RecallQuery::new("diet").with_limit(5),
        );
        assert!(results.is_empty());
    }

    #[test]
    fn test_engram_forget() {
        let engram = Engram::in_memory().unwrap();
        let agent = engram.register_agent("TestAgent");

        let node = MemoryNode::new(agent.agent_id, MemoryType::Semantic, "forgettable");
        let id = engram.store(node).unwrap();

        assert!(engram.storage.get(&id).is_ok());
        engram.forget(&id).unwrap();
        assert!(engram.storage.get(&id).is_err());
    }

    #[test]
    fn test_engram_link() {
        let engram = Engram::in_memory().unwrap();
        let agent = engram.register_agent("TestAgent");

        let node1 = MemoryNode::new(agent.agent_id, MemoryType::Semantic, "fact A");
        let node2 = MemoryNode::new(agent.agent_id, MemoryType::Semantic, "fact B");

        let id1 = engram.store(node1).unwrap();
        let id2 = engram.store(node2).unwrap();

        engram.link(id1, id2, LinkType::Semantic, 0.85).unwrap();

        let neighbors = engram.graph.neighbors(&id1);
        assert!(!neighbors.is_empty());
    }
}
