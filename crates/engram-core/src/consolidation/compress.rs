//! Memory summarization and compression triggers.
//!
//! When a cluster of episodic memories has been consolidated into
//! a semantic fact, the original episodic memories can be compressed
//! (archived) to save space.

use crate::memory::types::*;

/// Determine if a memory should be compressed/archived
pub fn should_compress(memory: &MemoryNode, has_been_consolidated: bool) -> bool {
    match &memory.memory_type {
        MemoryType::Episodic => {
            // Compress if consolidated AND retrievability is low
            has_been_consolidated && memory.retrievability() < 0.3
        }
        MemoryType::Working { ttl_seconds } => {
            // Working memory expires by TTL
            let elapsed = (chrono::Utc::now() - memory.temporal.created_at).num_seconds();
            elapsed > *ttl_seconds as i64
        }
        _ => false,
    }
}

/// Create a compressed summary from multiple episodic memories.
/// In production, this would use an LLM. Here we concatenate content.
pub fn create_summary(memories: &[MemoryNode]) -> String {
    if memories.is_empty() {
        return String::new();
    }

    if memories.len() == 1 {
        return memories[0].content.to_string();
    }

    // Simple heuristic: take unique content fragments
    let contents: Vec<&str> = memories.iter().map(|m| m.content.as_str()).collect();

    // Dedup very similar strings
    let mut unique = Vec::new();
    for content in &contents {
        if !unique.iter().any(|u: &&str| {
            let overlap = content.split_whitespace()
                .filter(|w| u.contains(w))
                .count();
            let total = content.split_whitespace().count().max(1);
            overlap as f32 / total as f32 > 0.8
        }) {
            unique.push(*content);
        }
    }

    format!("Consolidated from {} memories: {}", memories.len(), unique.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_should_compress_episodic() {
        let agent = Uuid::now_v7();
        let mut mem = MemoryNode::new(agent, MemoryType::Episodic, "test");
        // Make it old so retrievability is low
        mem.strength.last_review = chrono::Utc::now() - chrono::Duration::days(30);
        mem.strength.stability = 0.5;

        assert!(should_compress(&mem, true));
        assert!(!should_compress(&mem, false));
    }

    #[test]
    fn test_create_summary() {
        let agent = Uuid::now_v7();
        let mems = vec![
            MemoryNode::new(agent, MemoryType::Episodic, "ordered salad"),
            MemoryNode::new(agent, MemoryType::Episodic, "asked for vegetarian menu"),
            MemoryNode::new(agent, MemoryType::Episodic, "declined steak offer"),
        ];

        let summary = create_summary(&mems);
        assert!(summary.contains("Consolidated from 3 memories"));
    }
}
