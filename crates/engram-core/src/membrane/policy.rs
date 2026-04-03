//! Predefined access control policies for common scenarios.

use crate::memory::types::*;

/// Create a fully open policy — all memories visible to all agents
pub fn open_policy() -> PermeabilityPolicy {
    PermeabilityPolicy {
        default_visibility: Visibility::Global,
        share_rules: vec![],
        blocked_agents: Default::default(),
    }
}

/// Create a fully private policy — no sharing
pub fn private_policy() -> PermeabilityPolicy {
    PermeabilityPolicy {
        default_visibility: Visibility::Private,
        share_rules: vec![],
        blocked_agents: Default::default(),
    }
}

/// Create a selective sharing policy — share specific types with specific agents
pub fn selective_policy(
    targets: Vec<AgentId>,
    shared_types: Vec<MemoryTypeFilter>,
) -> PermeabilityPolicy {
    PermeabilityPolicy {
        default_visibility: Visibility::Private,
        share_rules: vec![ShareRule {
            target: ShareTarget::Group(targets),
            memory_types: shared_types,
            min_strength: 0.0,
            tags_filter: None,
            read_only: true,
        }],
        blocked_agents: Default::default(),
    }
}

/// Create a tag-based sharing policy — only share memories with specific tags
pub fn tag_based_policy(
    targets: Vec<AgentId>,
    required_tags: Vec<Tag>,
) -> PermeabilityPolicy {
    PermeabilityPolicy {
        default_visibility: Visibility::Private,
        share_rules: vec![ShareRule {
            target: ShareTarget::Group(targets),
            memory_types: vec![MemoryTypeFilter::All],
            min_strength: 0.0,
            tags_filter: Some(required_tags),
            read_only: true,
        }],
        blocked_agents: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_policy() {
        let policy = open_policy();
        assert_eq!(policy.default_visibility, Visibility::Global);
    }

    #[test]
    fn test_private_policy() {
        let policy = private_policy();
        assert_eq!(policy.default_visibility, Visibility::Private);
        assert!(policy.share_rules.is_empty());
    }
}
