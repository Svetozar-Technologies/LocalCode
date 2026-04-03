use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use engram_core::*;
use std::sync::Arc;

fn setup_engram(n: usize) -> Engram {
    let engram = Engram::in_memory().unwrap();
    let ns = engram.register_agent("BenchAgent");

    for i in 0..n {
        let node = MemoryNode::new(
            ns.agent_id,
            MemoryType::Semantic,
            format!("Memory fact number {} about various topics including science technology art history", i),
        );
        engram.store(node).unwrap();
    }

    engram
}

fn bench_store(c: &mut Criterion) {
    let engram = Engram::in_memory().unwrap();
    let ns = engram.register_agent("StoreAgent");

    c.bench_function("store_single", |b| {
        b.iter(|| {
            let node = MemoryNode::new(
                ns.agent_id,
                MemoryType::Semantic,
                "benchmark test memory content about various topics",
            );
            black_box(engram.store(node).unwrap());
        });
    });
}

fn bench_recall(c: &mut Criterion) {
    let mut group = c.benchmark_group("recall_latency");

    for size in &[100, 1000, 10_000] {
        let engram = setup_engram(*size);
        let _ns = engram.register_agent("RecallBench");

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                black_box(engram.recall(
                    RecallQuery::new("science technology facts")
                        .with_limit(10),
                ));
            });
        });
    }

    group.finish();
}

fn bench_hnsw_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("hnsw_search");

    for size in &[100, 1000, 10_000] {
        let hnsw = HnswIndex::new(384, storage::hnsw::HnswConfig::default());
        let mut rng = rand::thread_rng();

        for _ in 0..*size {
            let id = uuid::Uuid::now_v7();
            let vec: Vec<f32> = (0..384).map(|_| rand::Rng::gen::<f32>(&mut rng)).collect();
            hnsw.insert(id, vec).unwrap();
        }

        let query: Vec<f32> = (0..384).map(|_| rand::Rng::gen::<f32>(&mut rng)).collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                black_box(hnsw.search(&query, 10).unwrap());
            });
        });
    }

    group.finish();
}

fn bench_bm25_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("bm25_search");

    for size in &[100, 1000, 10_000] {
        let mut index = KeywordIndex::new();
        let contents = [
            "user is vegetarian and allergic to nuts",
            "booked a flight to paris for next week",
            "prefers dark mode in all applications",
            "works as a software engineer at a startup",
            "enjoys hiking and outdoor activities on weekends",
        ];

        for i in 0..*size {
            let id = uuid::Uuid::now_v7();
            index.add(id, contents[i % contents.len()]);
        }

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                black_box(index.search("vegetarian food preferences", 10));
            });
        });
    }

    group.finish();
}

fn bench_concurrent_store(c: &mut Criterion) {
    c.bench_function("concurrent_store_10_threads", |b| {
        b.iter(|| {
            let engram = Arc::new(Engram::in_memory().unwrap());
            let mut handles = Vec::new();

            for i in 0..10 {
                let engram = engram.clone();
                handles.push(std::thread::spawn(move || {
                    let ns = engram.register_agent(format!("ConcAgent{}", i));
                    for j in 0..10 {
                        let node = MemoryNode::new(
                            ns.agent_id,
                            MemoryType::Semantic,
                            format!("concurrent fact {} {}", i, j),
                        );
                        engram.store(node).unwrap();
                    }
                }));
            }

            for h in handles {
                h.join().unwrap();
            }

            assert_eq!(engram.storage.count(), 100);
        });
    });
}

criterion_group!(
    benches,
    bench_store,
    bench_recall,
    bench_hnsw_search,
    bench_bm25_search,
    bench_concurrent_store,
);
criterion_main!(benches);
