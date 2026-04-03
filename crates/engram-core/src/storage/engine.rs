use crate::memory::types::*;
use crate::storage::wal::{WalOp, WriteAheadLog};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Unified storage engine for Engram.
///
/// Combines:
/// - In-memory DashMap for fast concurrent reads
/// - Write-ahead log for crash safety
/// - Secondary indices (by agent, by type, by tag)
///
/// The engine is the single source of truth for all MemoryNodes.
pub struct StorageEngine {
    /// Primary store: MemoryId -> MemoryNode
    memories: DashMap<MemoryId, MemoryNode>,
    /// Secondary index: AgentId -> set of MemoryIds
    agent_index: DashMap<AgentId, HashSet<MemoryId>>,
    /// Secondary index: tag -> set of MemoryIds
    tag_index: DashMap<String, HashSet<MemoryId>>,
    /// Write-ahead log
    wal: RwLock<WriteAheadLog>,
    /// Base directory for data files
    data_dir: PathBuf,
    /// Statistics
    stats: Arc<EngineStats>,
}

#[derive(Debug, Default)]
pub struct EngineStats {
    pub store_count: std::sync::atomic::AtomicU64,
    pub read_count: std::sync::atomic::AtomicU64,
    pub delete_count: std::sync::atomic::AtomicU64,
}

impl StorageEngine {
    /// Create or open a storage engine at the given directory
    pub fn open(data_dir: impl AsRef<Path>) -> EngramResult<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&data_dir)?;

        let wal_path = data_dir.join("engram.wal");
        let wal = WriteAheadLog::open(&wal_path)?;

        let engine = Self {
            memories: DashMap::new(),
            agent_index: DashMap::new(),
            tag_index: DashMap::new(),
            wal: RwLock::new(wal),
            data_dir,
            stats: Arc::new(EngineStats::default()),
        };

        // Replay WAL to recover state
        engine.recover()?;

        Ok(engine)
    }

    /// Create an in-memory engine (for testing)
    pub fn in_memory() -> EngramResult<Self> {
        let dir = std::env::temp_dir().join(format!("engram-{}", uuid::Uuid::now_v7()));
        Self::open(dir)
    }

    /// Store a memory node
    pub fn store(&self, node: MemoryNode) -> EngramResult<MemoryId> {
        let id = node.id;

        // WAL first for durability
        self.wal.write().log_store(&node)?;

        // Update secondary indices
        self.index_memory(&node);

        // Primary store
        self.memories.insert(id, node);

        self.stats.store_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(id)
    }

    /// Retrieve a memory by ID
    pub fn get(&self, id: &MemoryId) -> EngramResult<MemoryNode> {
        self.stats.read_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        self.memories
            .get(id)
            .map(|r| r.value().clone())
            .ok_or(EngramError::NotFound(*id))
    }

    /// Update a memory node in place
    pub fn update(&self, node: MemoryNode) -> EngramResult<()> {
        let id = node.id;

        if !self.memories.contains_key(&id) {
            return Err(EngramError::NotFound(id));
        }

        self.wal.write().log_update(&node)?;

        // Re-index
        self.deindex_memory(&id);
        self.index_memory(&node);

        self.memories.insert(id, node);
        Ok(())
    }

    /// Delete (forget) a memory
    pub fn delete(&self, id: &MemoryId) -> EngramResult<()> {
        if !self.memories.contains_key(id) {
            return Err(EngramError::NotFound(*id));
        }

        self.wal.write().log_delete(id)?;

        self.deindex_memory(id);
        self.memories.remove(id);

        self.stats.delete_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(())
    }

    /// Get all memories for an agent
    pub fn get_by_agent(&self, agent_id: &AgentId) -> Vec<MemoryNode> {
        self.agent_index
            .get(agent_id)
            .map(|ids| {
                ids.value()
                    .iter()
                    .filter_map(|id| self.memories.get(id).map(|r| r.value().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get memories by tag
    pub fn get_by_tag(&self, tag: &str) -> Vec<MemoryNode> {
        self.tag_index
            .get(tag)
            .map(|ids| {
                ids.value()
                    .iter()
                    .filter_map(|id| self.memories.get(id).map(|r| r.value().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all memories matching a filter
    pub fn query(&self, filter: &MemoryFilter) -> Vec<MemoryNode> {
        let candidates: Vec<MemoryNode> = if let Some(agent_id) = &filter.agent_id {
            self.get_by_agent(agent_id)
        } else {
            self.memories.iter().map(|r| r.value().clone()).collect()
        };

        candidates
            .into_iter()
            .filter(|m| filter.matches(m))
            .collect()
    }

    /// Record access to a memory (updates strength/temporal)
    pub fn record_access(&self, id: &MemoryId) -> EngramResult<()> {
        if let Some(mut entry) = self.memories.get_mut(id) {
            entry.value_mut().record_access();
            Ok(())
        } else {
            Err(EngramError::NotFound(*id))
        }
    }

    /// Get total memory count
    pub fn count(&self) -> usize {
        self.memories.len()
    }

    /// Get memory count for an agent
    pub fn count_by_agent(&self, agent_id: &AgentId) -> usize {
        self.agent_index
            .get(agent_id)
            .map(|ids| ids.value().len())
            .unwrap_or(0)
    }

    /// Get all memory IDs
    pub fn all_ids(&self) -> Vec<MemoryId> {
        self.memories.iter().map(|r| *r.key()).collect()
    }

    /// Iterate over all memories
    pub fn iter(&self) -> impl Iterator<Item = MemoryNode> + '_ {
        self.memories.iter().map(|r| r.value().clone())
    }

    /// Compact: truncate WAL and rebuild indices
    pub fn compact(&self) -> EngramResult<()> {
        self.wal.write().truncate()?;

        // Re-log all current state
        let mut wal = self.wal.write();
        for entry in self.memories.iter() {
            wal.log_store(entry.value())?;
        }

        Ok(())
    }

    /// Write a checkpoint marker to the WAL.
    /// After indices are persisted, only entries after this marker need replay.
    pub fn checkpoint(&self) -> EngramResult<()> {
        self.wal.write().log_checkpoint()
    }

    pub fn stats(&self) -> &EngineStats {
        &self.stats
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    // ── Private helpers ──────────────────────────────────────────────────

    fn recover(&self) -> EngramResult<()> {
        let entries = self.wal.read().replay()?;
        let mut count = 0;

        for entry in entries {
            match entry.op {
                WalOp::Store | WalOp::Update => {
                    let node: MemoryNode = bincode::deserialize(&entry.payload)
                        .map_err(|e| EngramError::Serialization(e.to_string()))?;
                    self.index_memory(&node);
                    self.memories.insert(node.id, node);
                    count += 1;
                }
                WalOp::Delete => {
                    let id: MemoryId = bincode::deserialize(&entry.payload)
                        .map_err(|e| EngramError::Serialization(e.to_string()))?;
                    self.deindex_memory(&id);
                    self.memories.remove(&id);
                }
                WalOp::Checkpoint => {
                    // Checkpoint markers are informational; no action needed during replay
                }
            }
        }

        if count > 0 {
            tracing::info!("Recovered {} memories from WAL", count);
        }

        Ok(())
    }

    fn index_memory(&self, node: &MemoryNode) {
        // Agent index
        self.agent_index
            .entry(node.agent_id)
            .or_default()
            .insert(node.id);

        // Tag index
        for tag in &node.tags {
            self.tag_index
                .entry(tag.to_string())
                .or_default()
                .insert(node.id);
        }
    }

    fn deindex_memory(&self, id: &MemoryId) {
        if let Some(node) = self.memories.get(id) {
            // Remove from agent index — O(1) with HashSet
            if let Some(mut ids) = self.agent_index.get_mut(&node.agent_id) {
                ids.value_mut().remove(id);
            }

            // Remove from tag index — O(1) with HashSet
            for tag in &node.tags {
                if let Some(mut ids) = self.tag_index.get_mut(tag.as_str()) {
                    ids.value_mut().remove(id);
                }
            }
        }
    }
}

/// Filter for querying memories
#[derive(Debug, Default)]
pub struct MemoryFilter {
    pub agent_id: Option<AgentId>,
    pub memory_types: Option<Vec<MemoryTypeFilter>>,
    pub tags: Option<Vec<String>>,
    pub min_strength: Option<f32>,
    pub time_range: Option<TimeRange>,
    pub visibility: Option<Visibility>,
}

impl MemoryFilter {
    pub fn for_agent(agent_id: AgentId) -> Self {
        Self {
            agent_id: Some(agent_id),
            ..Default::default()
        }
    }

    pub fn matches(&self, memory: &MemoryNode) -> bool {
        // Check memory type
        if let Some(ref types) = self.memory_types {
            if !types.iter().any(|t| t.matches(&memory.memory_type)) {
                return false;
            }
        }

        // Check tags
        if let Some(ref required_tags) = self.tags {
            if !required_tags.iter().any(|t| {
                memory.tags.iter().any(|mt| mt.as_str() == t.as_str())
            }) {
                return false;
            }
        }

        // Check strength
        if let Some(min) = self.min_strength {
            if memory.retrievability() < min {
                return false;
            }
        }

        // Check time range
        if let Some(ref range) = self.time_range {
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

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_engine_store_and_get() {
        let engine = StorageEngine::in_memory().unwrap();
        let agent = Uuid::now_v7();
        let node = MemoryNode::new(agent, MemoryType::Semantic, "test fact");
        let id = node.id;

        engine.store(node).unwrap();
        let retrieved = engine.get(&id).unwrap();
        assert_eq!(retrieved.content.as_str(), "test fact");
    }

    #[test]
    fn test_engine_delete() {
        let engine = StorageEngine::in_memory().unwrap();
        let agent = Uuid::now_v7();
        let node = MemoryNode::new(agent, MemoryType::Semantic, "ephemeral");
        let id = node.id;

        engine.store(node).unwrap();
        assert_eq!(engine.count(), 1);

        engine.delete(&id).unwrap();
        assert_eq!(engine.count(), 0);
        assert!(engine.get(&id).is_err());
    }

    #[test]
    fn test_engine_agent_index() {
        let engine = StorageEngine::in_memory().unwrap();
        let agent1 = Uuid::now_v7();
        let agent2 = Uuid::now_v7();

        engine.store(MemoryNode::new(agent1, MemoryType::Semantic, "a1 fact")).unwrap();
        engine.store(MemoryNode::new(agent1, MemoryType::Semantic, "a1 fact 2")).unwrap();
        engine.store(MemoryNode::new(agent2, MemoryType::Semantic, "a2 fact")).unwrap();

        assert_eq!(engine.get_by_agent(&agent1).len(), 2);
        assert_eq!(engine.get_by_agent(&agent2).len(), 1);
    }

    #[test]
    fn test_engine_tag_index() {
        let engine = StorageEngine::in_memory().unwrap();
        let agent = Uuid::now_v7();

        let node = MemoryNode::new(agent, MemoryType::Semantic, "vegetarian fact")
            .tag("diet")
            .tag("preference");

        engine.store(node).unwrap();

        assert_eq!(engine.get_by_tag("diet").len(), 1);
        assert_eq!(engine.get_by_tag("preference").len(), 1);
        assert_eq!(engine.get_by_tag("nonexistent").len(), 0);
    }

    #[test]
    fn test_engine_update() {
        let engine = StorageEngine::in_memory().unwrap();
        let agent = Uuid::now_v7();

        let mut node = MemoryNode::new(agent, MemoryType::Semantic, "original");
        let id = node.id;
        engine.store(node.clone()).unwrap();

        node.content = "updated".into();
        engine.update(node).unwrap();

        let retrieved = engine.get(&id).unwrap();
        assert_eq!(retrieved.content.as_str(), "updated");
    }

    #[test]
    fn test_engine_filter() {
        let engine = StorageEngine::in_memory().unwrap();
        let agent = Uuid::now_v7();

        engine.store(MemoryNode::new(agent, MemoryType::Semantic, "fact")).unwrap();
        engine.store(MemoryNode::new(agent, MemoryType::Episodic, "action")).unwrap();

        let filter = MemoryFilter {
            agent_id: Some(agent),
            memory_types: Some(vec![MemoryTypeFilter::Semantic]),
            ..Default::default()
        };

        let results = engine.query(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content.as_str(), "fact");
    }

    #[test]
    fn test_engine_recovery() {
        let dir = tempfile::TempDir::new().unwrap();
        let agent = Uuid::now_v7();
        let id;

        {
            let engine = StorageEngine::open(dir.path()).unwrap();
            let node = MemoryNode::new(agent, MemoryType::Semantic, "persistent fact");
            id = node.id;
            engine.store(node).unwrap();
        }

        // Reopen — should recover from WAL
        let engine = StorageEngine::open(dir.path()).unwrap();
        let recovered = engine.get(&id).unwrap();
        assert_eq!(recovered.content.as_str(), "persistent fact");
    }
}
