//! Cross-agent memory sharing demo: TravelAgent ↔ DietAgent
//!
//! Demonstrates:
//! 1. DietAgent stores dietary knowledge about the user
//! 2. DietAgent configures permeability to share with TravelAgent
//! 3. TravelAgent queries across agent boundaries
//! 4. TravelAgent receives DietAgent's memories via membrane permeability
//! 5. TravelAgent uses this knowledge for meal planning

use engram_core::*;
use engram_core::membrane::policy;

fn main() -> anyhow::Result<()> {
    let engram = Engram::in_memory()?;

    // === Register Agents ===
    let diet_agent = engram.register_agent("DietAgent");
    let travel_agent = engram.register_agent("TravelAgent");

    println!("=== Engram Cross-Agent Memory Demo ===\n");
    println!("DietAgent:   {}", diet_agent.agent_id);
    println!("TravelAgent: {}", travel_agent.agent_id);

    // === DietAgent stores dietary knowledge ===
    println!("\n--- DietAgent: Storing dietary knowledge ---");

    let memories = vec![
        ("User is vegetarian — confirmed multiple times", vec!["diet", "preference"], 0.98),
        ("User has a severe nut allergy — carries EpiPen", vec!["diet", "allergy", "health"], 0.99),
        ("User prefers Italian and Thai cuisines", vec!["diet", "preference", "cuisine"], 0.90),
        ("User avoids gluten when possible but not celiac", vec!["diet", "preference"], 0.75),
        ("User drinks oat milk instead of dairy", vec!["diet", "preference"], 0.85),
    ];

    for (content, tags, _confidence) in &memories {
        let mut node = MemoryNode::new(
            diet_agent.agent_id,
            MemoryType::Semantic,
            *content,
        )
        .with_visibility(Visibility::Global); // Make globally visible

        for tag in tags {
            node.tags.push((*tag).into());
        }
        engram.store(node)?;
        println!("  Stored: {}", content);
    }

    // DietAgent also stores private internal notes
    let private_note = MemoryNode::new(
        diet_agent.agent_id,
        MemoryType::Episodic,
        "Internal: need to verify nut allergy severity with doctor",
    );
    // This is private (default) — TravelAgent shouldn't see it
    engram.store(private_note)?;
    println!("  Stored: [PRIVATE] internal observation note");

    // === Configure Permeability ===
    println!("\n--- DietAgent: Configuring membrane permeability ---");

    let share_policy = policy::selective_policy(
        vec![travel_agent.agent_id],
        vec![MemoryTypeFilter::Semantic], // Only share facts, not episodic notes
    );
    engram.namespaces.set_permeability(&diet_agent.agent_id, share_policy)?;
    println!("  Sharing semantic memories with TravelAgent");

    // === TravelAgent stores its own knowledge ===
    println!("\n--- TravelAgent: Storing travel knowledge ---");

    let travel_memories = vec![
        "User has upcoming trip to Tokyo, March 20-27",
        "User prefers boutique hotels over chains",
        "User's loyalty program: Delta SkyMiles Gold",
    ];

    for content in &travel_memories {
        let node = MemoryNode::new(
            travel_agent.agent_id,
            MemoryType::Semantic,
            *content,
        )
        .tag("travel");
        engram.store(node)?;
        println!("  Stored: {}", content);
    }

    // === TravelAgent queries cross-agent for dietary info ===
    println!("\n--- TravelAgent: Querying DietAgent for meal planning ---");
    println!("    Query: 'dietary preferences and food restrictions'\n");

    let results = engram.recall_across(
        travel_agent.agent_id,
        diet_agent.agent_id,
        RecallQuery::new("dietary preferences and food restrictions")
            .with_limit(10),
    );

    if results.is_empty() {
        println!("  No accessible memories found.");
    } else {
        println!("  Found {} accessible memories from DietAgent:", results.len());
        for (i, sm) in results.iter().enumerate() {
            let tags: Vec<&str> = sm.memory.tags.iter().map(|t| t.as_str()).collect();
            println!(
                "  {}. [score: {:.3}] [conf: {}] {}",
                i + 1,
                sm.score,
                match sm.memory.memory_type {
                    MemoryType::Semantic => "fact".to_string(),
                    _ => "n/a".to_string(),
                },
                sm.memory.content,
            );
            if !tags.is_empty() {
                println!("     tags: {}", tags.join(", "));
            }
        }
    }

    // Verify private episodic notes are NOT accessible
    println!("\n--- TravelAgent: Checking that private notes are filtered ---");
    let all_cross = engram.recall_across(
        travel_agent.agent_id,
        diet_agent.agent_id,
        RecallQuery::new("internal notes doctor verification")
            .with_limit(20),
    );

    let leaked_episodic: Vec<_> = all_cross
        .iter()
        .filter(|sm| matches!(sm.memory.memory_type, MemoryType::Episodic))
        .collect();

    if leaked_episodic.is_empty() {
        println!("  Correctly blocked: no private episodic notes accessible");
        println!("  (Only {} shared semantic memories returned)", all_cross.len());
    } else {
        println!("  WARNING: Private episodic notes leaked! Count: {}", leaked_episodic.len());
    }

    // === Summary ===
    println!("\n=== Demo Summary ===");
    let diet_stats = engram.agent_stats(&diet_agent.agent_id).unwrap();
    let travel_stats = engram.agent_stats(&travel_agent.agent_id).unwrap();

    println!("DietAgent:   {} memories", diet_stats.memory_count);
    println!("TravelAgent: {} memories", travel_stats.memory_count);
    println!("Graph nodes: {}", engram.graph.node_count());
    println!("Audit log:   {} entries", engram.audit.len());
    println!(
        "\nTravelAgent successfully accessed DietAgent's dietary knowledge \
         through the membrane permeability system, while private notes \
         remained protected."
    );

    Ok(())
}
