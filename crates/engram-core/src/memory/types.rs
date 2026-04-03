use chrono::{DateTime, Utc};
use compact_str::CompactString;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::collections::HashSet;
use uuid::Uuid;

// ============================================================================
// Core Identity Types
// ============================================================================

/// Time-sortable UUID v7 for memory identification
pub type MemoryId = Uuid;

/// Agent identifier
pub type AgentId = Uuid;

/// Tag for Zettelkasten-style categorization
pub type Tag = CompactString;

/// Embedding vector (384-dim for MiniLM-L6-v2)
pub type Embedding = Vec<f32>;

/// Unix timestamp in milliseconds
pub type Timestamp = DateTime<Utc>;

// ============================================================================
// Memory Node — the fundamental unit of Engram
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryNode {
    pub id: MemoryId,
    pub agent_id: AgentId,
    pub memory_type: MemoryType,
    pub content: CompactString,
    pub embedding: Option<Embedding>,
    pub metadata: MemoryMetadata,
    pub temporal: TemporalInfo,
    pub strength: MemoryStrength,
    pub visibility: Visibility,
    pub tags: SmallVec<[Tag; 4]>,
    pub links: SmallVec<[MemoryLink; 8]>,
}

impl MemoryNode {
    pub fn new(
        agent_id: AgentId,
        memory_type: MemoryType,
        content: impl Into<CompactString>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            agent_id,
            memory_type,
            content: content.into(),
            embedding: None,
            metadata: MemoryMetadata::default(),
            temporal: TemporalInfo::new(now),
            strength: MemoryStrength::new(now),
            visibility: Visibility::Private,
            tags: SmallVec::new(),
            links: SmallVec::new(),
        }
    }

    /// Record an access, updating temporal info and reinforcing memory strength
    pub fn record_access(&mut self) {
        let now = Utc::now();
        self.temporal.last_accessed = now;
        self.temporal.access_count += 1;
        self.strength.reinforce(now);
    }

    /// Calculate current retrievability based on time elapsed
    pub fn retrievability(&self) -> f32 {
        self.strength.retrievability_at(Utc::now())
    }

    /// Add a tag
    pub fn tag(mut self, tag: impl Into<CompactString>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set visibility
    pub fn with_visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Add embedding
    pub fn with_embedding(mut self, embedding: Embedding) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Add a link to another memory
    pub fn link_to(mut self, target: MemoryId, link_type: LinkType, weight: f32) -> Self {
        self.links.push(MemoryLink {
            target,
            link_type,
            weight,
        });
        self
    }
}

// ============================================================================
// Memory Types — cognitive science-inspired classification
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MemoryType {
    /// Event-based memories: "I booked flight X on date Y"
    Episodic,
    /// Fact-based knowledge: "User is vegetarian"
    Semantic,
    /// Skills and workflows: "To book flights, use API X"
    Procedural,
    /// Future intentions/plans: "Remind user to check-in 24h before"
    Prospective { trigger: Trigger },
    /// Short-term context buffer (automatically expires)
    Working { ttl_seconds: u64 },
}

impl MemoryType {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Episodic => "episodic",
            Self::Semantic => "semantic",
            Self::Procedural => "procedural",
            Self::Prospective { .. } => "prospective",
            Self::Working { .. } => "working",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Trigger {
    /// Fire at a specific time
    Temporal(Timestamp),
    /// Fire when a condition keyword appears in context
    Contextual(CompactString),
}

// ============================================================================
// Temporal Information
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemporalInfo {
    pub created_at: Timestamp,
    pub last_accessed: Timestamp,
    /// Fact validity window start (Zep-inspired)
    pub valid_from: Option<Timestamp>,
    /// Fact validity window end
    pub valid_until: Option<Timestamp>,
    pub access_count: u32,
}

impl TemporalInfo {
    pub fn new(now: Timestamp) -> Self {
        Self {
            created_at: now,
            last_accessed: now,
            valid_from: None,
            valid_until: None,
            access_count: 0,
        }
    }

    /// Check if the memory is temporally valid at a given time
    pub fn is_valid_at(&self, time: Timestamp) -> bool {
        if let Some(from) = self.valid_from {
            if time < from {
                return false;
            }
        }
        if let Some(until) = self.valid_until {
            if time > until {
                return false;
            }
        }
        true
    }
}

// ============================================================================
// Memory Strength — Ebbinghaus decay model
// ============================================================================

/// Decay constant for the Ebbinghaus forgetting curve
const DECAY_CONSTANT: f32 = 1.0;
/// How much stability increases on each reinforcement
const REINFORCEMENT_FACTOR: f32 = 0.3;
/// Below this retrievability, memory moves to archive
pub const ARCHIVE_THRESHOLD: f32 = 0.05;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryStrength {
    /// Base strength: 0.0 (forgotten) → 1.0 (vivid)
    pub current: f32,
    /// Resistance to decay — increases with spaced repetition
    pub stability: f32,
    /// Last time this memory was reviewed/accessed
    pub last_review: Timestamp,
}

impl MemoryStrength {
    pub fn new(now: Timestamp) -> Self {
        Self {
            current: 1.0,
            stability: 1.0,
            last_review: now,
        }
    }

    /// Calculate retrievability at a given time using Ebbinghaus curve
    /// R(t) = e^(-t / (stability * DECAY_CONSTANT))
    pub fn retrievability_at(&self, now: Timestamp) -> f32 {
        let elapsed = (now - self.last_review).num_seconds().max(0) as f32;
        let hours = elapsed / 3600.0;
        (-hours / (self.stability * DECAY_CONSTANT)).exp()
    }

    /// Reinforce memory on access (spaced repetition)
    pub fn reinforce(&mut self, now: Timestamp) {
        let current_r = self.retrievability_at(now);
        // Stability increases more when retrievability is low (spaced repetition benefit)
        self.stability *= 1.0 + REINFORCEMENT_FACTOR * (1.0 - current_r);
        self.current = 1.0;
        self.last_review = now;
    }

    /// Check if memory should be archived
    pub fn should_archive(&self, now: Timestamp) -> bool {
        self.retrievability_at(now) < ARCHIVE_THRESHOLD
    }
}

// ============================================================================
// Memory Links — edges in the cognitive mesh
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryLink {
    pub target: MemoryId,
    pub link_type: LinkType,
    /// Link strength: 0.0 → 1.0
    pub weight: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LinkType {
    /// A caused B
    Causal,
    /// A happened before/after B
    Temporal,
    /// A is related to B in meaning
    Semantic,
    /// A conflicts with B
    Contradicts,
    /// A is a more specific version of B
    Refines,
    /// A was consolidated from B
    DerivedFrom,
    /// Link across agent namespaces
    CrossAgent,
}

impl LinkType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Causal => "causal",
            Self::Temporal => "temporal",
            Self::Semantic => "semantic",
            Self::Contradicts => "contradicts",
            Self::Refines => "refines",
            Self::DerivedFrom => "derived_from",
            Self::CrossAgent => "cross_agent",
        }
    }
}

// ============================================================================
// Visibility / Access Control
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Visibility {
    /// Only the owning agent can access
    Private,
    /// Specific agents can access
    Shared(Vec<AgentId>),
    /// All agents can read
    Global,
}

impl Visibility {
    pub fn can_access(&self, requester: &AgentId, owner: &AgentId) -> bool {
        if requester == owner {
            return true;
        }
        match self {
            Self::Private => false,
            Self::Shared(agents) => agents.contains(requester),
            Self::Global => true,
        }
    }
}

// ============================================================================
// Memory Metadata
// ============================================================================

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MemoryMetadata {
    /// Source of the memory (e.g., "user_input", "api_response", "consolidation")
    pub source: Option<CompactString>,
    /// Arbitrary key-value pairs
    pub extra: std::collections::HashMap<CompactString, CompactString>,
}

// ============================================================================
// Agent Namespace / Membrane Types
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentNamespace {
    pub agent_id: AgentId,
    pub display_name: String,
    pub permeability: PermeabilityPolicy,
    pub memory_count: u64,
    pub created_at: Timestamp,
}

impl AgentNamespace {
    pub fn new(display_name: impl Into<String>) -> Self {
        Self {
            agent_id: Uuid::now_v7(),
            display_name: display_name.into(),
            permeability: PermeabilityPolicy::default(),
            memory_count: 0,
            created_at: Utc::now(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PermeabilityPolicy {
    pub default_visibility: Visibility,
    pub share_rules: Vec<ShareRule>,
    pub blocked_agents: HashSet<AgentId>,
}

impl Default for PermeabilityPolicy {
    fn default() -> Self {
        Self {
            default_visibility: Visibility::Private,
            share_rules: Vec::new(),
            blocked_agents: HashSet::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShareRule {
    pub target: ShareTarget,
    pub memory_types: Vec<MemoryTypeFilter>,
    pub min_strength: f32,
    pub tags_filter: Option<Vec<Tag>>,
    pub read_only: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ShareTarget {
    Agent(AgentId),
    Group(Vec<AgentId>),
    Global,
}

/// Filter for memory types in share rules (simplified for matching)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MemoryTypeFilter {
    Episodic,
    Semantic,
    Procedural,
    Prospective,
    Working,
    All,
}

impl MemoryTypeFilter {
    pub fn matches(&self, memory_type: &MemoryType) -> bool {
        match self {
            Self::All => true,
            Self::Episodic => matches!(memory_type, MemoryType::Episodic),
            Self::Semantic => matches!(memory_type, MemoryType::Semantic),
            Self::Procedural => matches!(memory_type, MemoryType::Procedural),
            Self::Prospective => matches!(memory_type, MemoryType::Prospective { .. }),
            Self::Working => matches!(memory_type, MemoryType::Working { .. }),
        }
    }
}

// ============================================================================
// Query Types
// ============================================================================

#[derive(Clone, Debug, Default)]
pub struct RecallQuery {
    pub text: Option<String>,
    pub embedding: Option<Embedding>,
    pub agent_id: Option<AgentId>,
    pub memory_types: Option<Vec<MemoryTypeFilter>>,
    pub tags: Option<Vec<Tag>>,
    pub time_range: Option<TimeRange>,
    pub min_strength: Option<f32>,
    pub limit: usize,
    pub weights: ScoringWeights,
    /// Context memory IDs for graph proximity scoring
    pub context_ids: Option<Vec<MemoryId>>,
    /// Cursor for pagination (base64-encoded last result position)
    pub cursor: Option<String>,
    /// Temporal validity filter — "what was true at time T?"
    pub as_of_time: Option<Timestamp>,
}

impl RecallQuery {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            limit: 10,
            ..Default::default()
        }
    }

    pub fn with_agent(mut self, agent_id: AgentId) -> Self {
        self.agent_id = Some(agent_id);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_types(mut self, types: Vec<MemoryTypeFilter>) -> Self {
        self.memory_types = Some(types);
        self
    }

    pub fn with_time_range(mut self, range: TimeRange) -> Self {
        self.time_range = Some(range);
        self
    }

    pub fn with_min_strength(mut self, min: f32) -> Self {
        self.min_strength = Some(min);
        self
    }

    pub fn with_tags(mut self, tags: Vec<Tag>) -> Self {
        self.tags = Some(tags);
        self
    }

    pub fn with_context(mut self, context_ids: Vec<MemoryId>) -> Self {
        self.context_ids = Some(context_ids);
        self
    }
}

#[derive(Clone, Debug)]
pub struct TimeRange {
    pub start: Option<Timestamp>,
    pub end: Option<Timestamp>,
}

/// Configurable weights for hybrid scoring
#[derive(Clone, Debug)]
pub struct ScoringWeights {
    /// Vector similarity weight
    pub vector: f32,
    /// Temporal recency weight
    pub temporal: f32,
    /// Memory strength weight
    pub strength: f32,
    /// Graph proximity weight
    pub graph: f32,
    /// Keyword match weight
    pub keyword: f32,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            vector: 0.35,
            temporal: 0.15,
            strength: 0.20,
            graph: 0.20,
            keyword: 0.10,
        }
    }
}

/// A scored memory result from retrieval
#[derive(Clone, Debug)]
pub struct ScoredMemory {
    pub memory: MemoryNode,
    pub score: f32,
    pub score_breakdown: ScoreBreakdown,
}

#[derive(Clone, Debug, Default)]
pub struct ScoreBreakdown {
    pub vector_score: f32,
    pub temporal_score: f32,
    pub strength_score: f32,
    pub graph_score: f32,
    pub keyword_score: f32,
}

// ============================================================================
// Events
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MemoryEvent {
    Stored(MemoryId),
    Updated(MemoryId),
    Forgotten(MemoryId),
    Linked(MemoryId, MemoryId, LinkType),
    Consolidated(Vec<MemoryId>, MemoryId),
    Archived(MemoryId),
    Shared(MemoryId, AgentId, AgentId),
}

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum EngramError {
    #[error("Memory not found: {0}")]
    NotFound(MemoryId),

    #[error("Access denied: agent {requester} cannot access memory owned by {owner}")]
    AccessDenied {
        requester: AgentId,
        owner: AgentId,
    },

    #[error("Agent not found: {0}")]
    AgentNotFound(AgentId),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("WAL error: {0}")]
    Wal(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),
}

pub type EngramResult<T> = Result<T, EngramError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_node_creation() {
        let agent_id = Uuid::now_v7();
        let node = MemoryNode::new(
            agent_id,
            MemoryType::Semantic,
            "User is vegetarian",
        );

        assert_eq!(node.agent_id, agent_id);
        assert_eq!(node.content.as_str(), "User is vegetarian");
        assert_eq!(node.memory_type, MemoryType::Semantic);
        assert_eq!(node.visibility, Visibility::Private);
        assert!(node.tags.is_empty());
        assert!(node.links.is_empty());
    }

    #[test]
    fn test_memory_builder_pattern() {
        let agent_id = Uuid::now_v7();
        let target_id = Uuid::now_v7();
        let node = MemoryNode::new(agent_id, MemoryType::Episodic, "booked flight")
            .tag("travel")
            .tag("booking")
            .with_visibility(Visibility::Global)
            .link_to(target_id, LinkType::Causal, 0.8);

        assert_eq!(node.tags.len(), 2);
        assert_eq!(node.visibility, Visibility::Global);
        assert_eq!(node.links.len(), 1);
        assert_eq!(node.links[0].target, target_id);
    }

    #[test]
    fn test_memory_strength_decay() {
        let past = Utc::now() - chrono::Duration::hours(24);
        let strength = MemoryStrength::new(past);

        let r = strength.retrievability_at(Utc::now());
        // After 24 hours with stability=1.0, retrievability should be very low
        assert!(r < 0.1, "Expected low retrievability after 24h, got {}", r);
    }

    #[test]
    fn test_memory_strength_reinforcement() {
        let past = Utc::now() - chrono::Duration::hours(12);
        let mut strength = MemoryStrength::new(past);
        let old_stability = strength.stability;

        strength.reinforce(Utc::now());

        assert!(strength.stability > old_stability);
        assert_eq!(strength.current, 1.0);
    }

    #[test]
    fn test_visibility_access() {
        let owner = Uuid::now_v7();
        let friend = Uuid::now_v7();
        let stranger = Uuid::now_v7();

        // Owner always has access
        assert!(Visibility::Private.can_access(&owner, &owner));

        // Private denies others
        assert!(!Visibility::Private.can_access(&friend, &owner));

        // Shared allows specified agents
        let shared = Visibility::Shared(vec![friend]);
        assert!(shared.can_access(&friend, &owner));
        assert!(!shared.can_access(&stranger, &owner));

        // Global allows everyone
        assert!(Visibility::Global.can_access(&stranger, &owner));
    }

    #[test]
    fn test_temporal_validity() {
        let now = Utc::now();
        let mut temporal = TemporalInfo::new(now);

        // No bounds — always valid
        assert!(temporal.is_valid_at(now));

        // Set validity window
        temporal.valid_from = Some(now - chrono::Duration::hours(1));
        temporal.valid_until = Some(now + chrono::Duration::hours(1));

        assert!(temporal.is_valid_at(now));
        assert!(!temporal.is_valid_at(now - chrono::Duration::hours(2)));
        assert!(!temporal.is_valid_at(now + chrono::Duration::hours(2)));
    }

    #[test]
    fn test_memory_type_filter() {
        let semantic = MemoryType::Semantic;
        let episodic = MemoryType::Episodic;

        assert!(MemoryTypeFilter::Semantic.matches(&semantic));
        assert!(!MemoryTypeFilter::Semantic.matches(&episodic));
        assert!(MemoryTypeFilter::All.matches(&semantic));
        assert!(MemoryTypeFilter::All.matches(&episodic));
    }
}
