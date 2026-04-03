use crate::memory::types::*;
use arc_swap::ArcSwap;
use parking_lot::Mutex;
use rand::Rng;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::io::{Read as IoRead, Write as IoWrite};
use std::path::Path;
use std::sync::Arc;

fn check_dim(got: usize, expected: usize) -> EngramResult<()> {
    if got != expected {
        Err(EngramError::Embedding(format!(
            "dimension mismatch: expected {}, got {}",
            expected, got
        )))
    } else {
        Ok(())
    }
}

/// HNSW (Hierarchical Navigable Small World) vector index.
///
/// Uses arc-swap for read-write separation:
/// - Reads: lock-free atomic snapshot load
/// - Writes: mutex-serialized clone-and-swap
pub struct HnswIndex {
    /// Read snapshot — lock-free access via ArcSwap
    snapshot: ArcSwap<HnswInner>,
    /// Write serialization
    write_lock: Mutex<()>,
    config: HnswConfig,
    dim: usize,
}

#[derive(Clone)]
struct HnswInner {
    /// All vectors stored by memory ID
    vectors: HashMap<MemoryId, Vec<f32>>,
    /// Graph layers: layer -> node -> neighbors
    layers: Vec<HashMap<MemoryId, Vec<MemoryId>>>,
    /// Entry point for searches
    entry_point: Option<MemoryId>,
}

/// Serializable snapshot for persistence
#[derive(serde::Serialize, serde::Deserialize)]
struct HnswSnapshot {
    vectors: HashMap<MemoryId, Vec<f32>>,
    layers: Vec<HashMap<MemoryId, Vec<MemoryId>>>,
    entry_point: Option<MemoryId>,
    dim: usize,
}

#[derive(Clone, Debug)]
pub struct HnswConfig {
    pub m: usize,
    pub m0: usize,
    pub ef_construction: usize,
    pub ef_search: usize,
    pub ml: f64,
}

impl Default for HnswConfig {
    fn default() -> Self {
        let m = 16;
        Self {
            m,
            m0: m * 2,
            ef_construction: 200,
            ef_search: 50,
            ml: 1.0 / (m as f64).ln(),
        }
    }
}

impl HnswIndex {
    pub fn new(dim: usize, config: HnswConfig) -> Self {
        Self {
            snapshot: ArcSwap::from_pointee(HnswInner {
                vectors: HashMap::new(),
                layers: vec![HashMap::new()],
                entry_point: None,
            }),
            write_lock: Mutex::new(()),
            config,
            dim,
        }
    }

    pub fn for_minilm() -> Self {
        Self::new(384, HnswConfig::default())
    }

    /// Insert a vector for a memory ID
    pub fn insert(&self, id: MemoryId, vector: Vec<f32>) -> EngramResult<()> {
        check_dim(vector.len(), self.dim)?;

        let level = self.random_level();
        let _guard = self.write_lock.lock();
        let mut inner = (*self.snapshot.load_full()).clone();

        // Ensure enough layers exist
        while inner.layers.len() <= level {
            inner.layers.push(HashMap::new());
        }

        inner.vectors.insert(id, vector.clone());

        if let Some(ep) = inner.entry_point {
            let mut current_nearest = ep;
            let top_layer = inner.layers.len() - 1;

            // Traverse from top to insertion level
            for l in (level + 1..=top_layer).rev() {
                current_nearest = greedy_search_layer_inner(&inner, &vector, current_nearest, l);
            }

            // Insert and connect at each layer from level down to 0
            for l in (0..=level.min(inner.layers.len() - 1)).rev() {
                let neighbors = search_layer_inner(&inner, &vector, current_nearest, self.config.ef_construction, l);

                let max_conn = if l == 0 { self.config.m0 } else { self.config.m };
                let selected: Vec<MemoryId> = neighbors.into_iter().take(max_conn).collect();

                inner.layers[l].insert(id, selected.clone());

                for &neighbor in &selected {
                    let HnswInner { ref vectors, ref mut layers, .. } = inner;
                    let layer = &mut layers[l];
                    layer.entry(neighbor).or_default().push(id);

                    // Prune if over max connections
                    if let Some(edges) = layer.get_mut(&neighbor) {
                        if edges.len() > max_conn {
                            if let Some(nv) = vectors.get(&neighbor).cloned() {
                                edges.sort_by(|a, b| {
                                    let da = vectors.get(a).map(|v| cosine_distance(&nv, v)).unwrap_or(f32::MAX);
                                    let db = vectors.get(b).map(|v| cosine_distance(&nv, v)).unwrap_or(f32::MAX);
                                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                                });
                                edges.truncate(max_conn);
                            }
                        }
                    }
                }

                if !selected.is_empty() {
                    current_nearest = selected[0];
                }
            }
        } else {
            // First node
            for l in 0..=level.min(inner.layers.len() - 1) {
                inner.layers[l].insert(id, Vec::new());
            }
        }

        if inner.entry_point.is_none() {
            inner.entry_point = Some(id);
        }

        // Atomic swap — readers see the new version instantly
        self.snapshot.store(Arc::new(inner));

        Ok(())
    }

    /// Find k approximate nearest neighbors (lock-free read)
    pub fn search(&self, query: &[f32], k: usize) -> EngramResult<Vec<(MemoryId, f32)>> {
        check_dim(query.len(), self.dim)?;

        // Lock-free snapshot load
        let inner = self.snapshot.load();

        let ep = match inner.entry_point {
            Some(ep) => ep,
            None => return Ok(Vec::new()),
        };

        let top_layer = inner.layers.len() - 1;
        let mut current = ep;

        // Greedy search from top layer down to layer 1
        for l in (1..=top_layer).rev() {
            current = greedy_search_layer_inner(&inner, query, current, l);
        }

        // ef_search at layer 0
        let candidates = search_layer_inner(&inner, query, current, self.config.ef_search, 0);

        // Return top-k with similarities
        Ok(candidates
            .into_iter()
            .take(k)
            .map(|id| {
                let sim = inner.vectors
                    .get(&id)
                    .map(|v| cosine_similarity(query, v))
                    .unwrap_or(0.0);
                (id, sim)
            })
            .collect())
    }

    /// Remove a vector
    pub fn remove(&self, id: &MemoryId) {
        let _guard = self.write_lock.lock();
        let mut inner = (*self.snapshot.load_full()).clone();

        inner.vectors.remove(id);

        for layer in inner.layers.iter_mut() {
            if let Some(neighbors) = layer.remove(id) {
                for n in &neighbors {
                    if let Some(edges) = layer.get_mut(n) {
                        edges.retain(|e| e != id);
                    }
                }
            }
        }

        if inner.entry_point == Some(*id) {
            inner.entry_point = inner.layers[0].keys().next().copied();
        }

        self.snapshot.store(Arc::new(inner));
    }

    pub fn len(&self) -> usize {
        self.snapshot.load().vectors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.snapshot.load().vectors.is_empty()
    }

    pub fn contains(&self, id: &MemoryId) -> bool {
        self.snapshot.load().vectors.contains_key(id)
    }

    /// Save the index to a file using bincode serialization
    pub fn save(&self, path: impl AsRef<Path>) -> EngramResult<()> {
        let inner = self.snapshot.load();

        // Serialize vectors, layers, entry_point, dim, config
        let data = HnswSnapshot {
            vectors: inner.vectors.clone(),
            layers: inner.layers.clone(),
            entry_point: inner.entry_point,
            dim: self.dim,
        };

        let bytes = bincode::serialize(&data)
            .map_err(|e| EngramError::Serialization(e.to_string()))?;

        let mut file = std::fs::File::create(path.as_ref())?;
        file.write_all(&bytes)?;
        Ok(())
    }

    /// Load the index from a file
    pub fn load(path: impl AsRef<Path>) -> EngramResult<Self> {
        let mut file = std::fs::File::open(path.as_ref())?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;

        let snapshot: HnswSnapshot = bincode::deserialize(&bytes)
            .map_err(|e| EngramError::Serialization(e.to_string()))?;

        Ok(Self {
            snapshot: ArcSwap::from_pointee(HnswInner {
                vectors: snapshot.vectors,
                layers: snapshot.layers,
                entry_point: snapshot.entry_point,
            }),
            write_lock: Mutex::new(()),
            config: HnswConfig::default(),
            dim: snapshot.dim,
        })
    }

    fn random_level(&self) -> usize {
        let mut rng = rand::thread_rng();
        let r: f64 = rng.gen();
        (-r.ln() * self.config.ml).floor() as usize
    }
}

// ── Non-locking inner functions ─────────────────────────────────────────

fn greedy_search_layer_inner(inner: &HnswInner, query: &[f32], start: MemoryId, layer: usize) -> MemoryId {
    let mut current = start;
    let mut current_dist = inner.vectors.get(&current)
        .map(|v| cosine_distance(query, v))
        .unwrap_or(f32::MAX);

    loop {
        let mut changed = false;
        if let Some(neighbors) = inner.layers.get(layer).and_then(|l| l.get(&current)) {
            for &n in neighbors {
                if let Some(nv) = inner.vectors.get(&n) {
                    let d = cosine_distance(query, nv);
                    if d < current_dist {
                        current = n;
                        current_dist = d;
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }

    current
}

fn search_layer_inner(inner: &HnswInner, query: &[f32], start: MemoryId, ef: usize, layer: usize) -> Vec<MemoryId> {
    let start_dist = inner.vectors.get(&start)
        .map(|v| cosine_distance(query, v))
        .unwrap_or(f32::MAX);

    let mut candidates: BinaryHeap<std::cmp::Reverse<(OrdF32, MemoryId)>> = BinaryHeap::new();
    let mut results: BinaryHeap<(OrdF32, MemoryId)> = BinaryHeap::new();
    let mut visited: HashSet<MemoryId> = HashSet::new();

    candidates.push(std::cmp::Reverse((OrdF32(start_dist), start)));
    results.push((OrdF32(start_dist), start));
    visited.insert(start);

    while let Some(std::cmp::Reverse((OrdF32(c_dist), c_id))) = candidates.pop() {
        if let Some(&(OrdF32(worst_dist), _)) = results.peek() {
            if c_dist > worst_dist && results.len() >= ef {
                break;
            }
        }

        if let Some(neighbors) = inner.layers.get(layer).and_then(|l| l.get(&c_id)) {
            for &n in neighbors {
                if !visited.insert(n) {
                    continue;
                }

                if let Some(nv) = inner.vectors.get(&n) {
                    let d = cosine_distance(query, nv);

                    let should_add = if results.len() < ef {
                        true
                    } else if let Some(&(OrdF32(worst_dist), _)) = results.peek() {
                        d < worst_dist
                    } else {
                        true
                    };

                    if should_add {
                        candidates.push(std::cmp::Reverse((OrdF32(d), n)));
                        results.push((OrdF32(d), n));
                        if results.len() > ef {
                            results.pop();
                        }
                    }
                }
            }
        }
    }

    let mut sorted: Vec<_> = results.into_vec();
    sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    sorted.into_iter().map(|(_, id)| id).collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct OrdF32(f32);

impl Eq for OrdF32 {}

impl PartialOrd for OrdF32 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for OrdF32 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Cosine similarity between two vectors (1.0 = identical, 0.0 = orthogonal)
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

/// Cosine distance (1 - similarity)
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    1.0 - cosine_similarity(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn random_vector(dim: usize) -> Vec<f32> {
        let mut rng = rand::thread_rng();
        (0..dim).map(|_| rng.gen::<f32>()).collect()
    }

    #[test]
    fn test_hnsw_basic() {
        let index = HnswIndex::new(4, HnswConfig::default());

        let id1 = Uuid::now_v7();
        let id2 = Uuid::now_v7();
        let id3 = Uuid::now_v7();

        index.insert(id1, vec![1.0, 0.0, 0.0, 0.0]).unwrap();
        index.insert(id2, vec![0.9, 0.1, 0.0, 0.0]).unwrap();
        index.insert(id3, vec![0.0, 0.0, 1.0, 0.0]).unwrap();

        assert_eq!(index.len(), 3);

        let results = index.search(&[1.0, 0.0, 0.0, 0.0], 2).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].0, id1);
    }

    #[test]
    fn test_hnsw_larger() {
        let dim = 32;
        let index = HnswIndex::new(dim, HnswConfig::default());

        for _ in 0..100 {
            let id = Uuid::now_v7();
            let v = random_vector(dim);
            index.insert(id, v).unwrap();
        }

        assert_eq!(index.len(), 100);

        let query = random_vector(dim);
        let results = index.search(&query, 10).unwrap();
        assert!(results.len() <= 10);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_hnsw_remove() {
        let index = HnswIndex::new(4, HnswConfig::default());
        let id = Uuid::now_v7();

        index.insert(id, vec![1.0, 0.0, 0.0, 0.0]).unwrap();
        assert!(index.contains(&id));

        index.remove(&id);
        assert!(!index.contains(&id));
    }

    #[test]
    fn test_hnsw_dimension_mismatch() {
        let index = HnswIndex::new(4, HnswConfig::default());
        let id = Uuid::now_v7();

        // Insert with wrong dimension should return error, not panic
        let result = index.insert(id, vec![1.0, 0.0]);
        assert!(result.is_err());

        // Search with wrong dimension should return error, not panic
        index.insert(id, vec![1.0, 0.0, 0.0, 0.0]).unwrap();
        let result = index.search(&[1.0, 0.0], 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c).abs() < 1e-6);
    }
}
