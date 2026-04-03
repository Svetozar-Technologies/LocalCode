//! Causal chain tracking and analysis.
//!
//! Traces cause-and-effect relationships between memories,
//! enabling agents to understand "why did this happen?" by
//! following causal links backward.

use crate::memory::types::*;
use crate::mesh::graph::CognitiveGraph;

/// Trace the causal chain backward from an effect to its root cause(s)
pub fn trace_causes(graph: &CognitiveGraph, effect: &MemoryId, max_depth: usize) -> Vec<Vec<MemoryId>> {
    let mut chains = Vec::new();
    let mut stack: Vec<(MemoryId, Vec<MemoryId>)> = vec![(*effect, vec![*effect])];

    while let Some((current, path)) = stack.pop() {
        if path.len() > max_depth + 1 {
            continue;
        }

        let causes: Vec<_> = graph
            .neighbors_by_type(&current, &LinkType::Causal)
            .into_iter()
            .filter(|(id, _)| !path.contains(id)) // Prevent cycles
            .collect();

        if causes.is_empty() {
            // This is a root cause
            if path.len() > 1 {
                chains.push(path);
            }
        } else {
            for (cause_id, _) in causes {
                let mut new_path = path.clone();
                new_path.push(cause_id);
                stack.push((cause_id, new_path));
            }
        }
    }

    chains
}

/// Trace the causal chain forward from a cause to its effects
pub fn trace_effects(graph: &CognitiveGraph, cause: &MemoryId, max_depth: usize) -> Vec<MemoryId> {
    graph.follow_link_type(cause, &LinkType::Causal, max_depth)
}

/// Find the longest causal chain passing through a given memory
pub fn longest_chain_through(graph: &CognitiveGraph, memory: &MemoryId) -> Vec<MemoryId> {
    // Trace backward to roots
    let cause_chains = trace_causes(graph, memory, 10);

    // For each root, trace forward
    let mut longest = Vec::new();

    if cause_chains.is_empty() {
        // This memory might be a root cause itself
        let effects = trace_effects(graph, memory, 10);
        let mut chain = vec![*memory];
        chain.extend(effects);
        return chain;
    }

    for chain in cause_chains {
        if let Some(root) = chain.last() {
            let effects = trace_effects(graph, root, 10);
            let mut full_chain = vec![*root];
            full_chain.extend(effects);
            if full_chain.len() > longest.len() {
                longest = full_chain;
            }
        }
    }

    longest
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_trace_causes() {
        let graph = CognitiveGraph::new();

        let root = Uuid::now_v7();
        let mid = Uuid::now_v7();
        let effect = Uuid::now_v7();

        graph.add_link(root, mid, LinkType::Causal, 1.0);
        graph.add_link(mid, effect, LinkType::Causal, 1.0);

        let chains = trace_causes(&graph, &effect, 5);
        assert!(!chains.is_empty());
    }

    #[test]
    fn test_trace_effects() {
        let graph = CognitiveGraph::new();

        let cause = Uuid::now_v7();
        let e1 = Uuid::now_v7();
        let e2 = Uuid::now_v7();

        graph.add_link(cause, e1, LinkType::Causal, 1.0);
        graph.add_link(e1, e2, LinkType::Causal, 1.0);

        let effects = trace_effects(&graph, &cause, 5);
        assert!(effects.len() >= 1);
    }
}
