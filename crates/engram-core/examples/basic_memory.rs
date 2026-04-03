//! Basic memory store and retrieve example.

use engram_core::*;

fn main() -> anyhow::Result<()> {
    // Create an in-memory Engram instance
    let engram = Engram::in_memory()?;

    // Register an agent
    let agent = engram.register_agent("MyAgent");
    println!("Registered agent: {} ({})", agent.display_name, agent.agent_id);

    // Store some memories
    let fact1 = MemoryNode::new(
        agent.agent_id,
        MemoryType::Semantic,
        "User's favorite color is blue",
    )
    .tag("preference");

    let fact2 = MemoryNode::new(
        agent.agent_id,
        MemoryType::Semantic,
        "User is vegetarian and allergic to nuts",
    )
    .tag("diet")
    .tag("health");

    let action = MemoryNode::new(
        agent.agent_id,
        MemoryType::Episodic,
        "Booked flight AA123 to New York for March 15",
    )
    .tag("travel");

    let skill = MemoryNode::new(
        agent.agent_id,
        MemoryType::Procedural,
        "To book flights, use the Amadeus API with OAuth2 authentication",
    )
    .tag("api")
    .tag("travel");

    let _id1 = engram.store(fact1)?;
    let _id2 = engram.store(fact2)?;
    let id3 = engram.store(action)?;
    let id4 = engram.store(skill)?;

    println!("Stored 4 memories");

    // Link causally: booking flight was caused by knowing about travel API
    engram.link(id4, id3, LinkType::Causal, 0.7)?;

    // Recall memories
    println!("\n--- Recall: 'dietary preferences' ---");
    let results = engram.recall(
        RecallQuery::new("dietary preferences")
            .with_agent(agent.agent_id)
            .with_limit(5),
    );
    for sm in &results {
        println!("  [{:.3}] {}", sm.score, sm.memory.content);
    }

    println!("\n--- Recall: 'travel plans' ---");
    let results = engram.recall(
        RecallQuery::new("travel plans flight booking")
            .with_agent(agent.agent_id)
            .with_limit(5),
    );
    for sm in &results {
        println!("  [{:.3}] {}", sm.score, sm.memory.content);
    }

    println!("\n--- Recall by tag: 'health' ---");
    let results = engram.recall(
        RecallQuery::new("health information")
            .with_agent(agent.agent_id)
            .with_tags(vec!["health".into()])
            .with_limit(5),
    );
    for sm in &results {
        println!("  [{:.3}] {}", sm.score, sm.memory.content);
    }

    // Show stats
    let stats = engram.agent_stats(&agent.agent_id).unwrap();
    println!("\n--- Stats ---");
    println!("Agent: {}", stats.display_name);
    println!("Memory count: {}", stats.memory_count);
    println!("Graph nodes: {}", engram.graph.node_count());
    println!("Graph edges: {}", engram.graph.edge_count());

    Ok(())
}
