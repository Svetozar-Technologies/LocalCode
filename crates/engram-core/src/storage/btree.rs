use crate::memory::types::*;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;

/// Temporal B-tree index for efficient time-range queries.
///
/// Stores memory IDs indexed by creation timestamp, enabling
/// fast range scans for "memories from the last N hours" queries.
pub struct TemporalIndex {
    /// BTreeMap of timestamp -> MemoryId for ordered time access
    index: RwLock<BTreeMap<i64, Vec<MemoryId>>>,
    /// O(1) length counter
    count: AtomicUsize,
}

impl TemporalIndex {
    pub fn new() -> Self {
        Self {
            index: RwLock::new(BTreeMap::new()),
            count: AtomicUsize::new(0),
        }
    }

    /// Insert a memory into the temporal index
    pub fn insert(&self, id: MemoryId, timestamp: DateTime<Utc>) {
        let key = timestamp.timestamp_millis();
        self.index.write().entry(key).or_default().push(id);
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    /// Remove a memory from the temporal index
    pub fn remove(&self, id: &MemoryId, timestamp: DateTime<Utc>) {
        let key = timestamp.timestamp_millis();
        let mut index = self.index.write();
        if let Some(ids) = index.get_mut(&key) {
            let before = ids.len();
            ids.retain(|i| i != id);
            let removed = before - ids.len();
            if removed > 0 {
                self.count.fetch_sub(removed, Ordering::Relaxed);
            }
            if ids.is_empty() {
                index.remove(&key);
            }
        }
    }

    /// Query memories in a time range
    pub fn range_query(&self, start: Option<DateTime<Utc>>, end: Option<DateTime<Utc>>) -> Vec<MemoryId> {
        let index = self.index.read();

        let start_key = start.map(|t| t.timestamp_millis()).unwrap_or(i64::MIN);
        let end_key = end.map(|t| t.timestamp_millis()).unwrap_or(i64::MAX);

        index
            .range(start_key..=end_key)
            .flat_map(|(_, ids)| ids.iter().copied())
            .collect()
    }

    /// Get the N most recent memory IDs
    pub fn most_recent(&self, n: usize) -> Vec<MemoryId> {
        let index = self.index.read();
        index
            .iter()
            .rev()
            .flat_map(|(_, ids)| ids.iter().copied())
            .take(n)
            .collect()
    }

    /// Get the N oldest memory IDs
    pub fn oldest(&self, n: usize) -> Vec<MemoryId> {
        let index = self.index.read();
        index
            .iter()
            .flat_map(|(_, ids)| ids.iter().copied())
            .take(n)
            .collect()
    }

    /// Count total entries — O(1)
    pub fn len(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    pub fn is_empty(&self) -> bool {
        self.count.load(Ordering::Relaxed) == 0
    }
}

impl Default for TemporalIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_temporal_index_insert_and_query() {
        let index = TemporalIndex::new();
        let now = Utc::now();

        let id1 = Uuid::now_v7();
        let id2 = Uuid::now_v7();
        let id3 = Uuid::now_v7();

        index.insert(id1, now - chrono::Duration::hours(2));
        index.insert(id2, now - chrono::Duration::hours(1));
        index.insert(id3, now);

        // Full range
        let all = index.range_query(None, None);
        assert_eq!(all.len(), 3);

        // Last hour only
        let recent = index.range_query(
            Some(now - chrono::Duration::minutes(90)),
            None,
        );
        assert_eq!(recent.len(), 2);
    }

    #[test]
    fn test_most_recent() {
        let index = TemporalIndex::new();
        let now = Utc::now();

        for i in 0..5 {
            let id = Uuid::now_v7();
            index.insert(id, now + chrono::Duration::seconds(i));
        }

        let recent = index.most_recent(3);
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn test_temporal_remove() {
        let index = TemporalIndex::new();
        let now = Utc::now();
        let id = Uuid::now_v7();

        index.insert(id, now);
        assert_eq!(index.len(), 1);

        index.remove(&id, now);
        assert_eq!(index.len(), 0);
    }
}
