//! Contradiction detection and resolution.
//!
//! When new semantic facts are extracted, they may conflict with
//! existing knowledge. This module detects and resolves such conflicts.

use crate::memory::types::*;
use crate::mesh::graph::CognitiveGraph;
use crate::storage::hnsw::cosine_similarity;
use chrono::Utc;

/// Result of conflict detection
#[derive(Debug)]
pub struct ConflictReport {
    pub memory_a: MemoryId,
    pub memory_b: MemoryId,
    pub similarity: f32,
    pub resolution: ConflictResolution,
}

#[derive(Debug)]
pub enum ConflictResolution {
    /// Keep the more recent / stronger memory
    PreferA,
    PreferB,
    /// Both are valid (no real conflict)
    NoConflict,
    /// Needs human/LLM arbitration
    NeedsReview,
}

/// Detect potential contradictions between a new memory and existing ones.
///
/// Two memories are potentially contradictory if:
/// 1. They have high semantic similarity (discussing the same topic)
/// 2. But one negates or contradicts the other
///
/// Without an LLM, we use heuristic: very similar embeddings with
/// negation keywords suggest contradiction.
pub fn detect_conflicts(
    new_memory: &MemoryNode,
    existing: &[MemoryNode],
    similarity_threshold: f32,
) -> Vec<ConflictReport> {
    let new_emb = match &new_memory.embedding {
        Some(e) => e,
        None => return Vec::new(),
    };

    let mut reports = Vec::new();
    let now = Utc::now();

    for existing_mem in existing {
        if existing_mem.id == new_memory.id {
            continue;
        }

        let existing_emb = match &existing_mem.embedding {
            Some(e) => e,
            None => continue,
        };

        let sim = cosine_similarity(new_emb, existing_emb);

        if sim >= similarity_threshold {
            // High similarity — check for contradiction signals
            let resolution = resolve_conflict(new_memory, existing_mem, sim, now);
            reports.push(ConflictReport {
                memory_a: new_memory.id,
                memory_b: existing_mem.id,
                similarity: sim,
                resolution,
            });
        }
    }

    reports
}

/// Resolve a conflict between two memories
fn resolve_conflict(
    a: &MemoryNode,
    b: &MemoryNode,
    similarity: f32,
    now: chrono::DateTime<Utc>,
) -> ConflictResolution {
    // If very high similarity, they might just be duplicates
    if similarity > 0.95 {
        // Prefer the more recent one
        if a.temporal.created_at > b.temporal.created_at {
            return ConflictResolution::PreferA;
        } else {
            return ConflictResolution::PreferB;
        }
    }

    // Check negation heuristics
    let has_negation = contains_negation(&a.content) != contains_negation(&b.content);

    if has_negation {
        // Prefer higher strength and more recent
        let a_score = a.strength.retrievability_at(now) * 0.5
            + if a.temporal.created_at > b.temporal.created_at { 0.5 } else { 0.0 };
        let b_score = b.strength.retrievability_at(now) * 0.5
            + if b.temporal.created_at > a.temporal.created_at { 0.5 } else { 0.0 };

        if (a_score - b_score).abs() < 0.1 {
            ConflictResolution::NeedsReview
        } else if a_score > b_score {
            ConflictResolution::PreferA
        } else {
            ConflictResolution::PreferB
        }
    } else {
        ConflictResolution::NoConflict
    }
}

/// Simple negation detection heuristic
fn contains_negation(text: &str) -> bool {
    let lower = text.to_lowercase();
    let negation_words = ["not", "no", "never", "don't", "doesn't", "isn't", "aren't", "won't", "can't", "neither", "nor", "none"];
    negation_words.iter().any(|w| {
        lower.split_whitespace().any(|word| word == *w)
    })
}

/// Register detected contradictions in the graph
pub fn register_contradictions(graph: &CognitiveGraph, reports: &[ConflictReport]) {
    for report in reports {
        match report.resolution {
            ConflictResolution::PreferA | ConflictResolution::PreferB | ConflictResolution::NeedsReview => {
                graph.add_link(report.memory_a, report.memory_b, LinkType::Contradicts, report.similarity);
                graph.add_link(report.memory_b, report.memory_a, LinkType::Contradicts, report.similarity);
            }
            ConflictResolution::NoConflict => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_negation_detection() {
        assert!(contains_negation("User is not vegetarian"));
        assert!(contains_negation("User doesn't eat meat"));
        assert!(!contains_negation("User is vegetarian"));
    }
}
