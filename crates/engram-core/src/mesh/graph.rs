use crate::memory::types::*;
use parking_lot::RwLock;
use petgraph::stable_graph::{NodeIndex, StableDiGraph};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{Read as IoRead, Write as IoWrite};
use std::path::Path;

/// Cognitive Memory Mesh — the interconnected graph of all memories.
///
/// Unlike flat memory stores, the mesh captures relationships between
/// memories: causality, temporal ordering, semantic similarity,
/// contradictions, refinements, and cross-agent links.
pub struct CognitiveGraph {
    graph: RwLock<StableDiGraph<MemoryId, LinkType>>,
    node_map: RwLock<HashMap<MemoryId, NodeIndex>>,
    weights: RwLock<HashMap<(MemoryId, MemoryId), f32>>,
}

impl CognitiveGraph {
    pub fn new() -> Self {
        Self {
            graph: RwLock::new(StableDiGraph::new()),
            node_map: RwLock::new(HashMap::new()),
            weights: RwLock::new(HashMap::new()),
        }
    }

    /// Ensure a node exists in the graph
    pub fn add_node(&self, id: MemoryId) {
        let mut map = self.node_map.write();
        map.entry(id).or_insert_with(|| self.graph.write().add_node(id));
    }

    /// Add a directed edge between two memories
    pub fn add_link(&self, from: MemoryId, to: MemoryId, link_type: LinkType, weight: f32) {
        self.add_node(from);
        self.add_node(to);

        let map = self.node_map.read();
        if let (Some(&from_idx), Some(&to_idx)) = (map.get(&from), map.get(&to)) {
            self.graph.write().add_edge(from_idx, to_idx, link_type);
            self.weights.write().insert((from, to), weight);
        }
    }

    /// Remove a node and all its edges
    pub fn remove_node(&self, id: &MemoryId) {
        let mut map = self.node_map.write();
        if let Some(idx) = map.remove(id) {
            // StableDiGraph preserves NodeIndex values on removal,
            // so no need to rebuild the map
            self.graph.write().remove_node(idx);
            self.weights.write().retain(|(a, b), _| a != id && b != id);
        }
    }

    /// Get all neighbors of a memory with their link types
    pub fn neighbors(&self, id: &MemoryId) -> Vec<(MemoryId, LinkType, f32)> {
        let map = self.node_map.read();
        let graph = self.graph.read();
        let weights = self.weights.read();

        let idx = match map.get(id) {
            Some(&idx) => idx,
            None => return Vec::new(),
        };

        let mut results = Vec::new();

        // Outgoing edges
        for edge in graph.edges_directed(idx, Direction::Outgoing) {
            let target_id = graph[edge.target()];
            let weight = weights.get(&(*id, target_id)).copied().unwrap_or(1.0);
            results.push((target_id, edge.weight().clone(), weight));
        }

        // Incoming edges
        for edge in graph.edges_directed(idx, Direction::Incoming) {
            let source_id = graph[edge.source()];
            let weight = weights.get(&(source_id, *id)).copied().unwrap_or(1.0);
            results.push((source_id, edge.weight().clone(), weight));
        }

        results
    }

    /// Get neighbors filtered by link type
    pub fn neighbors_by_type(&self, id: &MemoryId, link_type: &LinkType) -> Vec<(MemoryId, f32)> {
        self.neighbors(id)
            .into_iter()
            .filter(|(_, lt, _)| lt == link_type)
            .map(|(id, _, w)| (id, w))
            .collect()
    }

    /// BFS traversal from a starting node, returning nodes within `max_depth` hops
    pub fn bfs(&self, start: &MemoryId, max_depth: usize) -> Vec<(MemoryId, usize)> {
        let map = self.node_map.read();
        let graph = self.graph.read();

        let start_idx = match map.get(start) {
            Some(&idx) => idx,
            None => return Vec::new(),
        };

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut results = Vec::new();

        visited.insert(start_idx);
        queue.push_back((start_idx, 0usize));

        while let Some((idx, depth)) = queue.pop_front() {
            let id = graph[idx];
            results.push((id, depth));

            if depth >= max_depth {
                continue;
            }

            for neighbor in graph.neighbors_undirected(idx) {
                if visited.insert(neighbor) {
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }

        results
    }

    /// Calculate graph proximity score between two nodes.
    /// Returns 1.0 for adjacent nodes, decaying with distance, 0.0 if disconnected.
    pub fn proximity_score(&self, a: &MemoryId, b: &MemoryId, max_hops: usize) -> f32 {
        let reachable = self.bfs(a, max_hops);
        for (id, depth) in reachable {
            if id == *b {
                return 1.0 / (1.0 + depth as f32);
            }
        }
        0.0
    }

    /// Find all nodes reachable by following only a specific link type
    pub fn follow_link_type(&self, start: &MemoryId, link_type: &LinkType, max_depth: usize) -> Vec<MemoryId> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut results = Vec::new();

        visited.insert(*start);
        queue.push_back((*start, 0usize));

        while let Some((current, depth)) = queue.pop_front() {
            if depth > 0 {
                results.push(current);
            }
            if depth >= max_depth {
                continue;
            }
            for (neighbor, _, _) in self.neighbors(&current) {
                if visited.insert(neighbor) {
                    // Check if the edge type matches
                    let matching = self.neighbors_by_type(&current, link_type);
                    if matching.iter().any(|(id, _)| *id == neighbor) {
                        queue.push_back((neighbor, depth + 1));
                    }
                }
            }
        }

        results
    }

    /// Detect contradictions: nodes linked with Contradicts edges
    pub fn find_contradictions(&self, id: &MemoryId) -> Vec<MemoryId> {
        self.neighbors_by_type(id, &LinkType::Contradicts)
            .into_iter()
            .map(|(id, _)| id)
            .collect()
    }

    /// Count total nodes
    pub fn node_count(&self) -> usize {
        self.graph.read().node_count()
    }

    /// Count total edges
    pub fn edge_count(&self) -> usize {
        self.graph.read().edge_count()
    }

    /// Save the graph to a file
    pub fn save(&self, path: impl AsRef<Path>) -> EngramResult<()> {
        let graph = self.graph.read();
        let weights = self.weights.read();

        // Serialize edges as (from, to, link_type, weight) tuples
        let mut edges = Vec::new();
        for idx in graph.node_indices() {
            let from_id = graph[idx];
            for edge in graph.edges_directed(idx, Direction::Outgoing) {
                let to_id = graph[edge.target()];
                let w = weights.get(&(from_id, to_id)).copied().unwrap_or(1.0);
                edges.push((from_id, to_id, edge.weight().clone(), w));
            }
        }

        let snapshot = GraphSnapshot { edges };
        let bytes = bincode::serialize(&snapshot)
            .map_err(|e| EngramError::Serialization(e.to_string()))?;
        let mut file = std::fs::File::create(path.as_ref())?;
        file.write_all(&bytes)?;
        Ok(())
    }

    /// Load the graph from a file
    pub fn load(path: impl AsRef<Path>) -> EngramResult<Self> {
        let mut file = std::fs::File::open(path.as_ref())?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;

        let snapshot: GraphSnapshot = bincode::deserialize(&bytes)
            .map_err(|e| EngramError::Serialization(e.to_string()))?;

        let graph = Self::new();
        for (from, to, link_type, weight) in snapshot.edges {
            graph.add_link(from, to, link_type, weight);
        }
        Ok(graph)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct GraphSnapshot {
    edges: Vec<(MemoryId, MemoryId, LinkType, f32)>,
}

impl Default for CognitiveGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_graph_basic() {
        let graph = CognitiveGraph::new();

        let a = Uuid::now_v7();
        let b = Uuid::now_v7();
        let c = Uuid::now_v7();

        graph.add_link(a, b, LinkType::Causal, 0.9);
        graph.add_link(b, c, LinkType::Temporal, 0.8);

        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);
    }

    #[test]
    fn test_neighbors() {
        let graph = CognitiveGraph::new();

        let a = Uuid::now_v7();
        let b = Uuid::now_v7();
        let c = Uuid::now_v7();

        graph.add_link(a, b, LinkType::Semantic, 0.9);
        graph.add_link(a, c, LinkType::Causal, 0.7);

        let neighbors = graph.neighbors(&a);
        assert_eq!(neighbors.len(), 2);
    }

    #[test]
    fn test_bfs() {
        let graph = CognitiveGraph::new();

        let a = Uuid::now_v7();
        let b = Uuid::now_v7();
        let c = Uuid::now_v7();
        let d = Uuid::now_v7();

        graph.add_link(a, b, LinkType::Causal, 1.0);
        graph.add_link(b, c, LinkType::Causal, 1.0);
        graph.add_link(c, d, LinkType::Causal, 1.0);

        let reachable = graph.bfs(&a, 2);
        // Should reach a (depth 0), b (depth 1), c (depth 2)
        assert!(reachable.len() >= 3);
        assert!(!reachable.iter().any(|(id, _)| *id == d)); // d is 3 hops away
    }

    #[test]
    fn test_proximity_score() {
        let graph = CognitiveGraph::new();

        let a = Uuid::now_v7();
        let b = Uuid::now_v7();
        let c = Uuid::now_v7();

        graph.add_link(a, b, LinkType::Semantic, 1.0);
        graph.add_link(b, c, LinkType::Semantic, 1.0);

        let ab_score = graph.proximity_score(&a, &b, 5);
        let ac_score = graph.proximity_score(&a, &c, 5);

        assert!(ab_score > ac_score, "Adjacent nodes should have higher proximity");
        assert_eq!(ab_score, 0.5); // 1 / (1 + 1)
        assert!(ac_score > 0.0);
    }

    #[test]
    fn test_contradictions() {
        let graph = CognitiveGraph::new();

        let a = Uuid::now_v7();
        let b = Uuid::now_v7();

        graph.add_link(a, b, LinkType::Contradicts, 1.0);

        let contradictions = graph.find_contradictions(&a);
        assert_eq!(contradictions.len(), 1);
        assert_eq!(contradictions[0], b);
    }
}
