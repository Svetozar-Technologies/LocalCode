//! Integration tests: full lifecycle, cross-agent, concurrent access, crash recovery.

use engram_core::*;
use std::sync::Arc;

#[test]
fn test_full_lifecycle() {
    let engram = Engram::in_memory().unwrap();

    // Register agents
    let diet_ns = engram.register_agent("DietAgent");
    let travel_ns = engram.register_agent("TravelAgent");

    // Store various memory types
    let semantic = MemoryNode::new(diet_ns.agent_id, MemoryType::Semantic, "User is vegetarian")
        .tag("diet")
        .with_visibility(Visibility::Global);
    let episodic = MemoryNode::new(diet_ns.agent_id, MemoryType::Episodic, "User ordered salad at restaurant")
        .tag("meal");
    let procedural = MemoryNode::new(diet_ns.agent_id, MemoryType::Procedural, "Always check for nut allergies before recommending food");

    let sem_id = engram.store(semantic).unwrap();
    let epi_id = engram.store(episodic).unwrap();
    let proc_id = engram.store(procedural).unwrap();

    // Link memories
    engram.link(epi_id, sem_id, LinkType::Refines, 0.8).unwrap();
    engram.link(proc_id, sem_id, LinkType::DerivedFrom, 0.9).unwrap();

    // Recall
    let results = engram.recall(
        RecallQuery::new("vegetarian diet preferences")
            .with_agent(diet_ns.agent_id)
            .with_limit(10),
    );
    assert!(!results.is_empty());

    // Cross-agent recall (global visibility)
    let cross = engram.recall_across(
        travel_ns.agent_id,
        diet_ns.agent_id,
        RecallQuery::new("food preferences").with_limit(5),
    );
    assert!(!cross.is_empty());
    assert!(cross[0].memory.content.as_str().contains("vegetarian"));

    // Private memories should not be visible cross-agent
    // The episodic and procedural memories are private, so cross-agent recall
    // should only return the global semantic memory
    let all_cross = engram.recall_across(
        travel_ns.agent_id,
        diet_ns.agent_id,
        RecallQuery::new("diet").with_limit(10),
    );
    for r in &all_cross {
        assert!(
            matches!(r.memory.visibility, Visibility::Global),
            "Cross-agent recall returned a non-Global memory: {:?}",
            r.memory.visibility,
        );
    }

    // Forget
    engram.forget(&epi_id).unwrap();
    assert!(engram.storage.get(&epi_id).is_err());
    assert!(engram.storage.get(&sem_id).is_ok());

    // Stats
    let stats = engram.agent_stats(&diet_ns.agent_id).unwrap();
    assert_eq!(stats.memory_count, 2); // semantic + procedural (episodic was deleted)
}

#[test]
fn test_batch_operations() {
    let engram = Engram::in_memory().unwrap();
    let ns = engram.register_agent("BatchAgent");

    let nodes: Vec<MemoryNode> = (0..20)
        .map(|i| MemoryNode::new(ns.agent_id, MemoryType::Semantic, format!("Fact number {}", i)))
        .collect();

    let ids = engram.store_batch(nodes).unwrap();
    assert_eq!(ids.len(), 20);

    // Batch recall
    let queries = vec![
        RecallQuery::new("Fact number 5").with_agent(ns.agent_id).with_limit(3),
        RecallQuery::new("Fact number 15").with_agent(ns.agent_id).with_limit(3),
    ];
    let batch_results = engram.recall_batch(queries);
    assert_eq!(batch_results.len(), 2);
}

#[test]
fn test_concurrent_access() {
    let engram = Arc::new(Engram::in_memory().unwrap());

    let mut handles = Vec::new();

    // Spawn 10 agents storing memories concurrently
    for i in 0..10 {
        let engram = engram.clone();
        let handle = std::thread::spawn(move || {
            let ns = engram.register_agent(format!("Agent{}", i));
            for j in 0..10 {
                let node = MemoryNode::new(
                    ns.agent_id,
                    MemoryType::Semantic,
                    format!("Agent {} fact {}", i, j),
                );
                engram.store(node).unwrap();
            }
            ns.agent_id
        });
        handles.push(handle);
    }

    let agent_ids: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // Verify all memories stored
    assert_eq!(engram.storage.count(), 100);

    // Concurrent reads
    let mut read_handles = Vec::new();
    for &agent_id in &agent_ids {
        let engram = engram.clone();
        let handle = std::thread::spawn(move || {
            let results = engram.recall(
                RecallQuery::new("fact").with_agent(agent_id).with_limit(20),
            );
            assert!(!results.is_empty());
            results.len()
        });
        read_handles.push(handle);
    }

    for h in read_handles {
        h.join().unwrap();
    }
}

#[test]
fn test_working_memory() {
    let engram = Engram::in_memory().unwrap();
    let ns = engram.register_agent("WorkingMemAgent");

    // Push working memory
    engram.push_working(ns.agent_id, "current task: plan dinner", 300);
    engram.push_working(ns.agent_id, "constraint: budget $50", 300);

    let ctx = engram.working_context(&ns.agent_id);
    assert!(ctx.is_some());
    let ctx_str = ctx.unwrap();
    assert!(ctx_str.contains("plan dinner"));
    assert!(ctx_str.contains("budget"));
}

#[test]
fn test_crash_recovery() {
    let dir = tempfile::TempDir::new().unwrap();
    let data_dir = dir.path().to_path_buf();

    let agent_id;
    let mem_id;

    // Write data and drop (simulating crash without graceful shutdown)
    {
        let engram = Engram::open(&data_dir).unwrap();
        let ns = engram.register_agent("RecoveryAgent");
        agent_id = ns.agent_id;
        let node = MemoryNode::new(agent_id, MemoryType::Semantic, "This must survive a crash");
        mem_id = engram.store(node).unwrap();
        assert!(engram.storage.get(&mem_id).is_ok());
    }

    // Reopen (should replay WAL)
    {
        let engram = Engram::open(&data_dir).unwrap();
        let recovered = engram.storage.get(&mem_id).unwrap();
        assert_eq!(recovered.content.as_str(), "This must survive a crash");
    }
}

#[test]
fn test_graph_traversal() {
    let engram = Engram::in_memory().unwrap();
    let ns = engram.register_agent("GraphAgent");

    // Create a chain of memories
    let a = engram.store(MemoryNode::new(ns.agent_id, MemoryType::Semantic, "Root concept")).unwrap();
    let b = engram.store(MemoryNode::new(ns.agent_id, MemoryType::Semantic, "Derived concept")).unwrap();
    let c = engram.store(MemoryNode::new(ns.agent_id, MemoryType::Semantic, "Further derived")).unwrap();

    engram.link(a, b, LinkType::Causal, 0.9).unwrap();
    engram.link(b, c, LinkType::Causal, 0.8).unwrap();

    // BFS traversal
    let reachable = engram.graph.bfs(&a, 2);
    assert!(reachable.len() >= 3);

    // Proximity score
    let score_ab = engram.graph.proximity_score(&a, &b, 5);
    let score_ac = engram.graph.proximity_score(&a, &c, 5);
    assert!(score_ab > score_ac);
}

#[test]
fn test_index_persistence() {
    let dir = tempfile::TempDir::new().unwrap();
    let data_dir = dir.path().to_path_buf();

    // Store and snapshot
    {
        let engram = Engram::open(&data_dir).unwrap();
        let ns = engram.register_agent("PersistAgent");
        for i in 0..5 {
            engram.store(MemoryNode::new(
                ns.agent_id,
                MemoryType::Semantic,
                format!("Persistent fact {}", i),
            )).unwrap();
        }
        engram.snapshot_indices().unwrap();
    }

    // Verify snapshot files exist
    assert!(data_dir.join("indices/hnsw.bin").exists());
    assert!(data_dir.join("indices/keyword.bin").exists());
    assert!(data_dir.join("indices/graph.bin").exists());
}

#[test]
fn test_memory_filtering() {
    let engram = Engram::in_memory().unwrap();
    let ns = engram.register_agent("FilterAgent");

    engram.store(
        MemoryNode::new(ns.agent_id, MemoryType::Semantic, "high confidence fact")
            .tag("important"),
    ).unwrap();
    engram.store(
        MemoryNode::new(ns.agent_id, MemoryType::Episodic, "something happened")
            .tag("event"),
    ).unwrap();

    // Filter by memory type
    let results = engram.recall(
        RecallQuery::new("fact")
            .with_agent(ns.agent_id)
            .with_types(vec![MemoryTypeFilter::Semantic])
            .with_limit(10),
    );
    for r in &results {
        assert!(matches!(r.memory.memory_type, MemoryType::Semantic));
    }

    // Filter by tag
    let results = engram.recall(
        RecallQuery::new("important things")
            .with_agent(ns.agent_id)
            .with_tags(vec!["important".into()])
            .with_limit(10),
    );
    for r in &results {
        assert!(r.memory.tags.iter().any(|t| t.as_str() == "important"));
    }
}
