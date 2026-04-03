//! Self-editing memory tiers (Letta/MemGPT-inspired).
//!
//! Three tiers with different access characteristics:
//! - Core: always in context, agent-editable, limited capacity
//! - Recall: indexed, fast retrieval, main storage
//! - Archival: compressed, cold storage, large capacity

use crate::memory::types::*;
use std::collections::HashSet;

/// Memory tier classification
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MemoryTier {
    /// Always loaded into agent context. Limited capacity (e.g., 20 items).
    /// Agent can directly edit these memories.
    Core,
    /// Standard indexed storage. Fast retrieval via hybrid search.
    Recall,
    /// Compressed cold storage. Slower access, larger capacity.
    Archival,
}

/// Manages tier assignments for an agent's memories
pub struct TierManager {
    core_ids: HashSet<MemoryId>,
    archival_ids: HashSet<MemoryId>,
    core_capacity: usize,
}

impl TierManager {
    pub fn new(core_capacity: usize) -> Self {
        Self {
            core_ids: HashSet::new(),
            archival_ids: HashSet::new(),
            core_capacity,
        }
    }

    /// Get the tier of a memory
    pub fn tier(&self, id: &MemoryId) -> MemoryTier {
        if self.core_ids.contains(id) {
            MemoryTier::Core
        } else if self.archival_ids.contains(id) {
            MemoryTier::Archival
        } else {
            MemoryTier::Recall
        }
    }

    /// Promote a memory to core tier. Returns evicted ID if at capacity.
    pub fn promote_to_core(&mut self, id: MemoryId) -> Option<MemoryId> {
        self.archival_ids.remove(&id);

        if self.core_ids.len() >= self.core_capacity {
            // Evict oldest from core (in practice, would evict least-accessed)
            let evicted = self.core_ids.iter().next().copied();
            if let Some(evicted_id) = evicted {
                self.core_ids.remove(&evicted_id);
                self.core_ids.insert(id);
                return Some(evicted_id);
            }
        }

        self.core_ids.insert(id);
        None
    }

    /// Demote a memory from core to recall
    pub fn demote_to_recall(&mut self, id: &MemoryId) {
        self.core_ids.remove(id);
    }

    /// Archive a memory (move to cold storage)
    pub fn archive(&mut self, id: MemoryId) {
        self.core_ids.remove(&id);
        self.archival_ids.insert(id);
    }

    /// Restore from archival to recall tier
    pub fn restore(&mut self, id: &MemoryId) {
        self.archival_ids.remove(id);
    }

    /// Get all core memory IDs (for context loading)
    pub fn core_memories(&self) -> &HashSet<MemoryId> {
        &self.core_ids
    }

    /// Get all archival memory IDs
    pub fn archival_memories(&self) -> &HashSet<MemoryId> {
        &self.archival_ids
    }

    pub fn core_count(&self) -> usize {
        self.core_ids.len()
    }

    pub fn archival_count(&self) -> usize {
        self.archival_ids.len()
    }
}

impl Default for TierManager {
    fn default() -> Self {
        Self::new(20)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_tier_promotion_demotion() {
        let mut mgr = TierManager::new(3);
        let id1 = Uuid::now_v7();
        let id2 = Uuid::now_v7();

        assert_eq!(mgr.tier(&id1), MemoryTier::Recall);

        mgr.promote_to_core(id1);
        assert_eq!(mgr.tier(&id1), MemoryTier::Core);

        mgr.demote_to_recall(&id1);
        assert_eq!(mgr.tier(&id1), MemoryTier::Recall);

        mgr.archive(id2);
        assert_eq!(mgr.tier(&id2), MemoryTier::Archival);

        mgr.restore(&id2);
        assert_eq!(mgr.tier(&id2), MemoryTier::Recall);
    }

    #[test]
    fn test_core_capacity_eviction() {
        let mut mgr = TierManager::new(2);
        let id1 = Uuid::now_v7();
        let id2 = Uuid::now_v7();
        let id3 = Uuid::now_v7();

        mgr.promote_to_core(id1);
        mgr.promote_to_core(id2);
        assert_eq!(mgr.core_count(), 2);

        // Third promotion should evict one
        let evicted = mgr.promote_to_core(id3);
        assert!(evicted.is_some());
        assert_eq!(mgr.core_count(), 2);
    }
}
