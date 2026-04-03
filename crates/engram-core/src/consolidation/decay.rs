use crate::memory::types::*;
use crate::storage::engine::StorageEngine;
use chrono::Utc;

/// Ebbinghaus forgetting curve implementation with spaced repetition.
///
/// Memories naturally decay over time. When retrieved, their stability
/// increases (spaced repetition effect). Memories below the archive
/// threshold are moved to cold storage.

/// Process decay for all memories, returning IDs that should be archived
pub fn process_decay(storage: &StorageEngine) -> Vec<MemoryId> {
    let now = Utc::now();
    let mut to_archive = Vec::new();

    for memory in storage.iter() {
        // Skip working memory (handled by TTL)
        if matches!(memory.memory_type, MemoryType::Working { .. }) {
            continue;
        }

        if memory.strength.should_archive(now) {
            to_archive.push(memory.id);
        }
    }

    to_archive
}

/// Calculate the optimal review interval for a memory (spaced repetition)
pub fn optimal_review_interval(strength: &MemoryStrength) -> chrono::Duration {
    // Higher stability = longer interval before next review needed
    let hours = (strength.stability * 24.0) as i64; // Base: 24h * stability
    chrono::Duration::hours(hours.max(1))
}

/// Get memories that are due for review (retrievability dropping below threshold)
pub fn memories_due_for_review(storage: &StorageEngine, threshold: f32) -> Vec<MemoryId> {
    let now = Utc::now();

    storage
        .iter()
        .filter(|m| {
            !matches!(m.memory_type, MemoryType::Working { .. })
                && m.strength.retrievability_at(now) < threshold
                && !m.strength.should_archive(now) // Don't review already-archivable
        })
        .map(|m| m.id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_process_decay() {
        let storage = StorageEngine::in_memory().unwrap();
        let agent = Uuid::now_v7();

        // Create a memory with artificially old last_review
        let mut node = MemoryNode::new(agent, MemoryType::Semantic, "old fact");
        node.strength.last_review = Utc::now() - chrono::Duration::days(30);
        node.strength.stability = 0.5;

        storage.store(node).unwrap();

        let to_archive = process_decay(&storage);
        assert!(!to_archive.is_empty(), "Old memory should be flagged for archival");
    }

    #[test]
    fn test_optimal_review_interval() {
        let now = Utc::now();
        let weak = MemoryStrength { current: 1.0, stability: 0.5, last_review: now };
        let strong = MemoryStrength { current: 1.0, stability: 5.0, last_review: now };

        let weak_interval = optimal_review_interval(&weak);
        let strong_interval = optimal_review_interval(&strong);

        assert!(strong_interval > weak_interval);
    }
}
