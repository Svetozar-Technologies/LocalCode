use crate::memory::types::*;
use dashmap::DashMap;

/// Agent namespace manager.
///
/// Each agent operates within its own namespace, providing isolation
/// by default. The membrane controller mediates cross-namespace access.
pub struct NamespaceManager {
    namespaces: DashMap<AgentId, AgentNamespace>,
}

impl NamespaceManager {
    pub fn new() -> Self {
        Self {
            namespaces: DashMap::new(),
        }
    }

    /// Register a new agent namespace
    pub fn register(&self, name: impl Into<String>) -> AgentNamespace {
        let ns = AgentNamespace::new(name);
        let id = ns.agent_id;
        self.namespaces.insert(id, ns.clone());
        ns
    }

    /// Register with a specific ID
    pub fn register_with_id(&self, id: AgentId, name: impl Into<String>) -> AgentNamespace {
        let mut ns = AgentNamespace::new(name);
        ns.agent_id = id;
        self.namespaces.insert(id, ns.clone());
        ns
    }

    /// Get a namespace by agent ID
    pub fn get(&self, agent_id: &AgentId) -> Option<AgentNamespace> {
        self.namespaces.get(agent_id).map(|r| r.value().clone())
    }

    /// Update permeability policy for an agent
    pub fn set_permeability(&self, agent_id: &AgentId, policy: PermeabilityPolicy) -> EngramResult<()> {
        match self.namespaces.get_mut(agent_id) {
            Some(mut ns) => {
                ns.permeability = policy;
                Ok(())
            }
            None => Err(EngramError::AgentNotFound(*agent_id)),
        }
    }

    /// Increment memory count for an agent
    pub fn increment_memory_count(&self, agent_id: &AgentId) {
        if let Some(mut ns) = self.namespaces.get_mut(agent_id) {
            ns.memory_count += 1;
        }
    }

    /// Decrement memory count for an agent
    pub fn decrement_memory_count(&self, agent_id: &AgentId) {
        if let Some(mut ns) = self.namespaces.get_mut(agent_id) {
            ns.memory_count = ns.memory_count.saturating_sub(1);
        }
    }

    /// List all registered agents
    pub fn list_agents(&self) -> Vec<AgentNamespace> {
        self.namespaces.iter().map(|r| r.value().clone()).collect()
    }

    /// Check if an agent exists
    pub fn exists(&self, agent_id: &AgentId) -> bool {
        self.namespaces.contains_key(agent_id)
    }

    /// Remove an agent namespace
    pub fn remove(&self, agent_id: &AgentId) -> Option<AgentNamespace> {
        self.namespaces.remove(agent_id).map(|(_, v)| v)
    }

    pub fn count(&self) -> usize {
        self.namespaces.len()
    }
}

impl Default for NamespaceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_agent() {
        let mgr = NamespaceManager::new();
        let ns = mgr.register("TravelAgent");

        assert_eq!(ns.display_name, "TravelAgent");
        assert_eq!(ns.memory_count, 0);
        assert!(mgr.exists(&ns.agent_id));
    }

    #[test]
    fn test_list_agents() {
        let mgr = NamespaceManager::new();
        mgr.register("Agent1");
        mgr.register("Agent2");

        let agents = mgr.list_agents();
        assert_eq!(agents.len(), 2);
    }

    #[test]
    fn test_update_permeability() {
        let mgr = NamespaceManager::new();
        let ns = mgr.register("Agent");

        let policy = PermeabilityPolicy {
            default_visibility: Visibility::Global,
            ..Default::default()
        };

        mgr.set_permeability(&ns.agent_id, policy).unwrap();

        let updated = mgr.get(&ns.agent_id).unwrap();
        assert_eq!(updated.permeability.default_visibility, Visibility::Global);
    }
}
