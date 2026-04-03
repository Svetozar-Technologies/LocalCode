use crate::memory::types::*;
use crate::membrane::namespace::NamespaceManager;

/// Permeability controller — determines what memories can cross agent boundaries.
///
/// Biology-inspired: cell membranes are selectively permeable, allowing
/// certain molecules through while blocking others. Similarly, agent
/// membranes selectively share memories based on configurable rules.
pub struct PermeabilityController<'a> {
    namespaces: &'a NamespaceManager,
}

impl<'a> PermeabilityController<'a> {
    pub fn new(namespaces: &'a NamespaceManager) -> Self {
        Self { namespaces }
    }

    /// Check if a requester agent can access a specific memory
    pub fn can_access(&self, requester: &AgentId, memory: &MemoryNode) -> bool {
        // Owner always has access
        if *requester == memory.agent_id {
            return true;
        }

        // Check owner's permeability policy
        if let Some(owner_ns) = self.namespaces.get(&memory.agent_id) {
            // Check if requester is blocked
            if owner_ns.permeability.blocked_agents.contains(requester) {
                return false;
            }

            // Check share rules (these override per-memory visibility)
            for rule in &owner_ns.permeability.share_rules {
                if self.rule_matches(rule, requester, memory) {
                    return true;
                }
            }

            // Check per-memory visibility
            if memory.visibility.can_access(requester, &memory.agent_id) {
                return true;
            }

            // Fall back to namespace default visibility
            match &owner_ns.permeability.default_visibility {
                Visibility::Global => true,
                Visibility::Shared(agents) => agents.contains(requester),
                Visibility::Private => false,
            }
        } else {
            // No namespace registered — deny by default
            false
        }
    }

    /// Filter a list of memories to only those accessible by the requester
    pub fn filter_accessible(&self, requester: &AgentId, memories: &[MemoryNode]) -> Vec<MemoryNode> {
        memories
            .iter()
            .filter(|m| self.can_access(requester, m))
            .cloned()
            .collect()
    }

    /// Get all agents that a specific memory can be shared with
    pub fn shareable_with(&self, memory: &MemoryNode) -> Vec<AgentId> {
        self.namespaces
            .list_agents()
            .into_iter()
            .filter(|ns| ns.agent_id != memory.agent_id)
            .filter(|ns| self.can_access(&ns.agent_id, memory))
            .map(|ns| ns.agent_id)
            .collect()
    }

    fn rule_matches(&self, rule: &ShareRule, requester: &AgentId, memory: &MemoryNode) -> bool {
        // Check target
        let target_matches = match &rule.target {
            ShareTarget::Agent(id) => id == requester,
            ShareTarget::Group(ids) => ids.contains(requester),
            ShareTarget::Global => true,
        };

        if !target_matches {
            return false;
        }

        // Check memory type filter
        if !rule.memory_types.iter().any(|f| f.matches(&memory.memory_type)) {
            return false;
        }

        // Check minimum strength
        if memory.retrievability() < rule.min_strength {
            return false;
        }

        // Check tag filter
        if let Some(ref required_tags) = rule.tags_filter {
            if !required_tags.iter().any(|t| memory.tags.iter().any(|mt| mt == t)) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_access_control() {
        let mgr = NamespaceManager::new();
        let travel = mgr.register("TravelAgent");
        let diet = mgr.register("DietAgent");

        // Diet agent creates a private memory
        let mem = MemoryNode::new(diet.agent_id, MemoryType::Semantic, "user is vegetarian");

        let ctrl = PermeabilityController::new(&mgr);

        // Diet agent can access its own memory
        assert!(ctrl.can_access(&diet.agent_id, &mem));
        // Travel agent cannot (private by default)
        assert!(!ctrl.can_access(&travel.agent_id, &mem));
    }

    #[test]
    fn test_global_visibility() {
        let mgr = NamespaceManager::new();
        let travel = mgr.register("TravelAgent");
        let diet = mgr.register("DietAgent");

        let mem = MemoryNode::new(diet.agent_id, MemoryType::Semantic, "user is vegetarian")
            .with_visibility(Visibility::Global);

        let ctrl = PermeabilityController::new(&mgr);
        assert!(ctrl.can_access(&travel.agent_id, &mem));
    }

    #[test]
    fn test_share_rules() {
        let mgr = NamespaceManager::new();
        let travel = mgr.register("TravelAgent");
        let diet = mgr.register("DietAgent");

        // Diet agent shares semantic memories with TravelAgent
        let policy = PermeabilityPolicy {
            default_visibility: Visibility::Private,
            share_rules: vec![ShareRule {
                target: ShareTarget::Agent(travel.agent_id),
                memory_types: vec![MemoryTypeFilter::Semantic],
                min_strength: 0.0,
                tags_filter: None,
                read_only: true,
            }],
            blocked_agents: Default::default(),
        };

        mgr.set_permeability(&diet.agent_id, policy).unwrap();

        let semantic_mem = MemoryNode::new(diet.agent_id, MemoryType::Semantic, "user is vegetarian");
        let episodic_mem = MemoryNode::new(diet.agent_id, MemoryType::Episodic, "ordered salad");

        let ctrl = PermeabilityController::new(&mgr);

        // Semantic memory is shared
        assert!(ctrl.can_access(&travel.agent_id, &semantic_mem));
        // Episodic memory is not (rule only covers semantic)
        assert!(!ctrl.can_access(&travel.agent_id, &episodic_mem));
    }

    #[test]
    fn test_blocked_agent() {
        let mgr = NamespaceManager::new();
        let travel = mgr.register("TravelAgent");
        let diet = mgr.register("DietAgent");

        let mut blocked = std::collections::HashSet::new();
        blocked.insert(travel.agent_id);

        let policy = PermeabilityPolicy {
            default_visibility: Visibility::Global,
            share_rules: vec![],
            blocked_agents: blocked,
        };

        mgr.set_permeability(&diet.agent_id, policy).unwrap();

        let mem = MemoryNode::new(diet.agent_id, MemoryType::Semantic, "secret")
            .with_visibility(Visibility::Global);

        let ctrl = PermeabilityController::new(&mgr);
        assert!(!ctrl.can_access(&travel.agent_id, &mem));
    }
}
