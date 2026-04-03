//! Multi-agent conversation demo with shared memory.

use engram_core::*;

fn main() -> anyhow::Result<()> {
    let engram = Engram::in_memory()?;

    // Register multiple agents
    let scheduler = engram.register_agent("SchedulerAgent");
    let finance = engram.register_agent("FinanceAgent");
    let health = engram.register_agent("HealthAgent");

    println!("=== Multi-Agent Conversation Demo ===\n");

    // SchedulerAgent learns about user's schedule
    let meeting = MemoryNode::new(
        scheduler.agent_id,
        MemoryType::Episodic,
        "User has a dentist appointment on March 22 at 2pm",
    )
    .tag("schedule")
    .tag("health")
    .with_visibility(Visibility::Global);
    engram.store(meeting)?;

    let deadline = MemoryNode::new(
        scheduler.agent_id,
        MemoryType::Episodic,
        "Project deadline is March 25 — user is stressed about it",
    )
    .tag("schedule")
    .tag("work")
    .with_visibility(Visibility::Global);
    engram.store(deadline)?;

    // FinanceAgent learns about expenses
    let expense = MemoryNode::new(
        finance.agent_id,
        MemoryType::Semantic,
        "User's monthly budget for dining out is $500",
    )
    .tag("finance")
    .tag("food")
    .with_visibility(Visibility::Global);
    engram.store(expense)?;

    // HealthAgent learns about health
    let health_mem = MemoryNode::new(
        health.agent_id,
        MemoryType::Semantic,
        "User should take vitamin D supplements daily",
    )
    .tag("health")
    .tag("medication")
    .with_visibility(Visibility::Global);
    engram.store(health_mem)?;

    // Each agent queries across boundaries
    println!("--- HealthAgent queries SchedulerAgent ---");
    let results = engram.recall_across(
        health.agent_id,
        scheduler.agent_id,
        RecallQuery::new("health appointments").with_limit(5),
    );
    for sm in &results {
        println!("  [score: {:.3}] {}", sm.score, sm.memory.content);
    }

    println!("\n--- FinanceAgent queries all ---");
    let all_results = engram.recall(
        RecallQuery::new("dining budget food expenses").with_limit(10),
    );
    for sm in &all_results {
        println!("  [score: {:.3}] [agent: {}] {}", sm.score, sm.memory.agent_id, sm.memory.content);
    }

    println!("\n=== System State ===");
    println!("Total memories: {}", engram.storage.count());
    println!("Agents: {}", engram.namespaces.count());

    Ok(())
}
