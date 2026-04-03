//! Audit trail for cross-agent memory access.
//!
//! Records every cross-agent access for security review and debugging.

use crate::memory::types::*;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub requester: AgentId,
    pub owner: AgentId,
    pub memory_id: MemoryId,
    pub action: AuditAction,
    pub granted: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AuditAction {
    Read,
    Query,
    Share,
}

pub struct AuditLog {
    entries: RwLock<VecDeque<AuditEntry>>,
    max_entries: usize,
}

impl AuditLog {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: RwLock::new(VecDeque::with_capacity(max_entries)),
            max_entries,
        }
    }

    /// Record an access attempt
    pub fn record(
        &self,
        requester: AgentId,
        owner: AgentId,
        memory_id: MemoryId,
        action: AuditAction,
        granted: bool,
    ) {
        let entry = AuditEntry {
            timestamp: Utc::now(),
            requester,
            owner,
            memory_id,
            action,
            granted,
        };

        let mut entries = self.entries.write();
        if entries.len() >= self.max_entries {
            entries.pop_front();
        }
        entries.push_back(entry);
    }

    /// Get recent entries
    pub fn recent(&self, count: usize) -> Vec<AuditEntry> {
        let entries = self.entries.read();
        entries.iter().rev().take(count).cloned().collect()
    }

    /// Get entries for a specific agent (as requester)
    pub fn by_requester(&self, agent_id: &AgentId) -> Vec<AuditEntry> {
        let entries = self.entries.read();
        entries
            .iter()
            .filter(|e| e.requester == *agent_id)
            .cloned()
            .collect()
    }

    /// Get denied access attempts
    pub fn denied_attempts(&self) -> Vec<AuditEntry> {
        let entries = self.entries.read();
        entries.iter().filter(|e| !e.granted).cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }

    pub fn clear(&self) {
        self.entries.write().clear();
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new(10_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_audit_logging() {
        let log = AuditLog::new(100);

        let requester = Uuid::now_v7();
        let owner = Uuid::now_v7();
        let memory_id = Uuid::now_v7();

        log.record(requester, owner, memory_id, AuditAction::Read, true);
        log.record(requester, owner, Uuid::now_v7(), AuditAction::Read, false);

        assert_eq!(log.len(), 2);
        assert_eq!(log.denied_attempts().len(), 1);
    }

    #[test]
    fn test_audit_capacity() {
        let log = AuditLog::new(3);

        for _ in 0..5 {
            log.record(Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7(), AuditAction::Query, true);
        }

        assert_eq!(log.len(), 3); // Oldest entries evicted
    }
}
