use crate::memory::types::*;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// LRU cache for recall query results with TTL-based expiry.
pub struct RecallCache {
    entries: Mutex<HashMap<CacheKey, CacheEntry>>,
    max_entries: usize,
    ttl: Duration,
}

#[derive(Clone, Hash, Eq, PartialEq)]
struct CacheKey {
    query_text: Option<String>,
    agent_id: Option<AgentId>,
    limit: usize,
}

struct CacheEntry {
    results: Vec<CachedResult>,
    created_at: Instant,
}

#[derive(Clone)]
struct CachedResult {
    memory_id: MemoryId,
    _score: f32,
}

impl RecallCache {
    pub fn new(max_entries: usize, ttl: Duration) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            max_entries,
            ttl,
        }
    }

    /// Try to get cached results for a query
    pub fn get(&self, query: &RecallQuery) -> Option<Vec<MemoryId>> {
        let key = Self::make_key(query);
        let entries = self.entries.lock();

        if let Some(entry) = entries.get(&key) {
            if entry.created_at.elapsed() < self.ttl {
                return Some(entry.results.iter().map(|r| r.memory_id).collect());
            }
        }

        None
    }

    /// Cache results for a query
    pub fn put(&self, query: &RecallQuery, results: &[ScoredMemory]) {
        let key = Self::make_key(query);
        let entry = CacheEntry {
            results: results
                .iter()
                .map(|sm| CachedResult {
                    memory_id: sm.memory.id,
                    _score: sm.score,
                })
                .collect(),
            created_at: Instant::now(),
        };

        let mut entries = self.entries.lock();

        // Evict expired entries if at capacity
        if entries.len() >= self.max_entries {
            entries.retain(|_, v| v.created_at.elapsed() < self.ttl);
        }

        // If still at capacity, remove oldest
        if entries.len() >= self.max_entries {
            if let Some(oldest_key) = entries
                .iter()
                .min_by_key(|(_, v)| v.created_at)
                .map(|(k, _)| k.clone())
            {
                entries.remove(&oldest_key);
            }
        }

        entries.insert(key, entry);
    }

    /// Invalidate cache entries for a specific agent
    pub fn invalidate_agent(&self, agent_id: &AgentId) {
        let mut entries = self.entries.lock();
        entries.retain(|k, _| k.agent_id.as_ref() != Some(agent_id));
    }

    /// Invalidate all cache entries
    pub fn invalidate_all(&self) {
        self.entries.lock().clear();
    }

    fn make_key(query: &RecallQuery) -> CacheKey {
        CacheKey {
            query_text: query.text.clone(),
            agent_id: query.agent_id,
            limit: query.limit,
        }
    }
}

impl Default for RecallCache {
    fn default() -> Self {
        Self::new(1000, Duration::from_secs(30))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_cache_hit_and_miss() {
        let cache = RecallCache::new(100, Duration::from_secs(60));
        let agent = Uuid::now_v7();

        let query = RecallQuery::new("test query").with_agent(agent).with_limit(5);

        // Miss
        assert!(cache.get(&query).is_none());

        // Store
        let node = MemoryNode::new(agent, MemoryType::Semantic, "test");
        let scored = vec![ScoredMemory {
            memory: node.clone(),
            score: 0.8,
            score_breakdown: ScoreBreakdown::default(),
        }];
        cache.put(&query, &scored);

        // Hit
        let cached = cache.get(&query);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 1);
    }

    #[test]
    fn test_cache_invalidation() {
        let cache = RecallCache::new(100, Duration::from_secs(60));
        let agent = Uuid::now_v7();

        let query = RecallQuery::new("test").with_agent(agent).with_limit(5);
        let node = MemoryNode::new(agent, MemoryType::Semantic, "test");
        let scored = vec![ScoredMemory {
            memory: node,
            score: 0.8,
            score_breakdown: ScoreBreakdown::default(),
        }];
        cache.put(&query, &scored);

        assert!(cache.get(&query).is_some());
        cache.invalidate_agent(&agent);
        assert!(cache.get(&query).is_none());
    }

    #[test]
    fn test_cache_ttl_expiry() {
        let cache = RecallCache::new(100, Duration::from_millis(1));
        let agent = Uuid::now_v7();

        let query = RecallQuery::new("test").with_agent(agent).with_limit(5);
        let node = MemoryNode::new(agent, MemoryType::Semantic, "test");
        let scored = vec![ScoredMemory {
            memory: node,
            score: 0.8,
            score_breakdown: ScoreBreakdown::default(),
        }];
        cache.put(&query, &scored);

        std::thread::sleep(Duration::from_millis(5));
        assert!(cache.get(&query).is_none());
    }
}
