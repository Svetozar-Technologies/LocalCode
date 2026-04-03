use super::types::*;
use chrono::{DateTime, Utc};

/// Check if a prospective memory should fire given current context
pub fn should_trigger(memory: &MemoryNode, context: &str, now: DateTime<Utc>) -> bool {
    if let MemoryType::Prospective { ref trigger } = memory.memory_type {
        match trigger {
            Trigger::Temporal(when) => now >= *when,
            Trigger::Contextual(keyword) => {
                context.to_lowercase().contains(keyword.to_lowercase().as_str())
            }
        }
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_contextual_trigger() {
        let agent = Uuid::now_v7();
        let mem = MemoryNode::new(
            agent,
            MemoryType::Prospective { trigger: Trigger::Contextual("food".into()) },
            "Mention nut allergy when discussing food",
        ).tag("health");

        assert!(should_trigger(&mem, "Let's talk about food options", Utc::now()));
        assert!(!should_trigger(&mem, "Let's discuss the weather", Utc::now()));
    }

    #[test]
    fn test_temporal_trigger() {
        let agent = Uuid::now_v7();
        let past = Utc::now() - chrono::Duration::hours(1);
        let mem = MemoryNode::new(
            agent,
            MemoryType::Prospective { trigger: Trigger::Temporal(past) },
            "Already due",
        );

        assert!(should_trigger(&mem, "", Utc::now()));

        let future = Utc::now() + chrono::Duration::hours(1);
        let mem2 = MemoryNode::new(
            agent,
            MemoryType::Prospective { trigger: Trigger::Temporal(future) },
            "Not yet",
        );
        assert!(!should_trigger(&mem2, "", Utc::now()));
    }
}
