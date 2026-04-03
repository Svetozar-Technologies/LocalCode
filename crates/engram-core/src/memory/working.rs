use super::types::*;
use chrono::Utc;
use compact_str::CompactString;
use std::collections::VecDeque;

/// Short-term working memory buffer.
///
/// Working memory holds the current conversation context, recent tool outputs,
/// and temporary state that expires after a configurable TTL. It acts as a
/// scratchpad that higher-level processes can read from.
pub struct WorkingMemoryBuffer {
    agent_id: AgentId,
    capacity: usize,
    buffer: VecDeque<MemoryNode>,
}

impl WorkingMemoryBuffer {
    pub fn new(agent_id: AgentId, capacity: usize) -> Self {
        Self {
            agent_id,
            capacity,
            buffer: VecDeque::with_capacity(capacity),
        }
    }

    /// Push a new working memory item. Evicts oldest if at capacity.
    pub fn push(&mut self, content: impl Into<CompactString>, ttl_seconds: u64) -> MemoryId {
        let node = MemoryNode::new(
            self.agent_id,
            MemoryType::Working { ttl_seconds },
            content,
        );
        let id = node.id;

        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(node);
        id
    }

    /// Get all non-expired items
    pub fn active_items(&self) -> Vec<&MemoryNode> {
        let now = Utc::now();
        self.buffer
            .iter()
            .filter(|node| {
                if let MemoryType::Working { ttl_seconds } = node.memory_type {
                    let elapsed = (now - node.temporal.created_at).num_seconds();
                    elapsed < ttl_seconds as i64
                } else {
                    false
                }
            })
            .collect()
    }

    /// Remove expired items
    pub fn gc(&mut self) -> usize {
        let now = Utc::now();
        let before = self.buffer.len();
        self.buffer.retain(|node| {
            if let MemoryType::Working { ttl_seconds } = node.memory_type {
                let elapsed = (now - node.temporal.created_at).num_seconds();
                elapsed < ttl_seconds as i64
            } else {
                false
            }
        });
        before - self.buffer.len()
    }

    /// Get buffer contents as a single context string
    pub fn context_string(&self) -> String {
        self.active_items()
            .iter()
            .map(|n| n.content.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

/// Create a working memory item (standalone, not in a buffer)
pub fn transient(agent_id: AgentId, content: impl Into<CompactString>, ttl_seconds: u64) -> MemoryNode {
    MemoryNode::new(agent_id, MemoryType::Working { ttl_seconds }, content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_working_memory_buffer() {
        let agent = Uuid::now_v7();
        let mut buf = WorkingMemoryBuffer::new(agent, 3);

        buf.push("first message", 300);
        buf.push("second message", 300);
        buf.push("third message", 300);

        assert_eq!(buf.len(), 3);
        assert_eq!(buf.active_items().len(), 3);

        // Overflow evicts oldest
        buf.push("fourth message", 300);
        assert_eq!(buf.len(), 3);
    }

    #[test]
    fn test_working_memory_context() {
        let agent = Uuid::now_v7();
        let mut buf = WorkingMemoryBuffer::new(agent, 10);
        buf.push("Hello", 300);
        buf.push("World", 300);

        let ctx = buf.context_string();
        assert!(ctx.contains("Hello"));
        assert!(ctx.contains("World"));
    }

    #[test]
    fn test_transient_memory() {
        let agent = Uuid::now_v7();
        let mem = transient(agent, "temporary context", 60);
        assert!(matches!(mem.memory_type, MemoryType::Working { ttl_seconds: 60 }));
    }
}
