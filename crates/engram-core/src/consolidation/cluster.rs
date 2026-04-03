use crate::memory::types::*;
use crate::storage::hnsw::cosine_similarity;
use std::collections::HashMap;

/// DBSCAN-inspired clustering for episodic memory consolidation.
///
/// Groups similar episodic memories together so they can be
/// consolidated into semantic facts.

/// Cluster episodic memories by embedding similarity
pub fn cluster_by_similarity(
    memories: &[(MemoryId, Vec<f32>)],
    eps: f32,
    min_points: usize,
) -> Vec<Vec<MemoryId>> {
    if memories.is_empty() {
        return Vec::new();
    }

    let n = memories.len();
    let mut labels: Vec<Option<usize>> = vec![None; n];
    let mut cluster_id = 0;

    for i in 0..n {
        if labels[i].is_some() {
            continue;
        }

        // Find neighbors
        let neighbors = region_query(memories, i, eps);

        if neighbors.len() < min_points {
            continue; // Noise point
        }

        // Start a new cluster
        labels[i] = Some(cluster_id);
        let mut seeds = neighbors;
        let mut j = 0;

        while j < seeds.len() {
            let q = seeds[j];
            j += 1;

            if labels[q].is_some() {
                continue;
            }

            labels[q] = Some(cluster_id);

            let q_neighbors = region_query(memories, q, eps);
            if q_neighbors.len() >= min_points {
                for n in q_neighbors {
                    if !seeds.contains(&n) {
                        seeds.push(n);
                    }
                }
            }
        }

        cluster_id += 1;
    }

    // Group by cluster
    let mut clusters: HashMap<usize, Vec<MemoryId>> = HashMap::new();
    for (i, label) in labels.iter().enumerate() {
        if let Some(cid) = label {
            clusters.entry(*cid).or_default().push(memories[i].0);
        }
    }

    clusters.into_values().collect()
}

/// Find all points within eps distance
fn region_query(
    memories: &[(MemoryId, Vec<f32>)],
    point_idx: usize,
    eps: f32,
) -> Vec<usize> {
    let point_vec = &memories[point_idx].1;
    memories
        .iter()
        .enumerate()
        .filter(|(i, (_, vec))| {
            *i != point_idx && cosine_similarity(point_vec, vec) >= (1.0 - eps)
        })
        .map(|(i, _)| i)
        .collect()
}

/// Simple content-based grouping (for memories without embeddings)
pub fn cluster_by_tags(
    memories: &[MemoryNode],
    min_shared_tags: usize,
) -> Vec<Vec<MemoryId>> {
    let mut groups: HashMap<Vec<String>, Vec<MemoryId>> = HashMap::new();

    for mem in memories {
        if mem.tags.is_empty() {
            continue;
        }
        let mut sorted_tags: Vec<String> = mem.tags.iter().map(|t| t.to_string()).collect();
        sorted_tags.sort();
        groups.entry(sorted_tags).or_default().push(mem.id);
    }

    groups
        .into_values()
        .filter(|g| g.len() >= min_shared_tags)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_cluster_similar_vectors() {
        let memories: Vec<(MemoryId, Vec<f32>)> = vec![
            (Uuid::now_v7(), vec![1.0, 0.0, 0.0]),
            (Uuid::now_v7(), vec![0.95, 0.05, 0.0]),
            (Uuid::now_v7(), vec![0.9, 0.1, 0.0]),
            (Uuid::now_v7(), vec![0.0, 0.0, 1.0]),
            (Uuid::now_v7(), vec![0.0, 0.05, 0.95]),
        ];

        let clusters = cluster_by_similarity(&memories, 0.2, 2);
        assert!(clusters.len() >= 1, "Should find at least one cluster");
    }

    #[test]
    fn test_cluster_empty() {
        let clusters = cluster_by_similarity(&[], 0.3, 2);
        assert!(clusters.is_empty());
    }
}
