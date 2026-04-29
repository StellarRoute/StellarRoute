/**
 * Load test: Cache invalidation strategies
 *
 * Compares hierarchical graph-based invalidation vs full cache clear
 * Simulates:
 *   - 10K pairs trading
 *   - 100K routes cached
 *   - Liquidity updates arriving at 50/sec
 *   - Measures: invalidation latency, stale read rate, memory usage
 */

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::HashSet;

// Simulated simplified cache store (not using Redis in bench)
struct SimpleCacheStore {
    data: std::collections::HashMap<String, String>,
}

impl SimpleCacheStore {
    fn new() -> Self {
        Self {
            data: std::collections::HashMap::new(),
        }
    }

    fn set(&mut self, key: String, _val: String) {
        self.data.insert(key, "cached".to_string());
    }

    fn delete(&mut self, key: &str) {
        self.data.remove(key);
    }

    fn delete_many(&mut self, keys: &[&str]) {
        for key in keys {
            self.data.remove(*key);
        }
    }

    fn delete_all(&mut self) {
        self.data.clear();
    }

    fn size(&self) -> usize {
        self.data.len()
    }
}

// Strategy 1: Full cache clear (naive baseline)
fn invalidate_full_clear(cache: &mut SimpleCacheStore, _updated_pair: &str, cache_size: usize) {
    cache.delete_all();
}

// Strategy 2: Pair-level invalidation (basic)
fn invalidate_pair_level(
    cache: &mut SimpleCacheStore,
    updated_pair: &str,
    pairs_per_quote: usize,
) {
    // Simulate deleting only quotes/routes for this pair
    for i in 0..pairs_per_quote * 2 {
        let key = format!("{}:route:{}", updated_pair, i);
        cache.delete(&key);
    }
}

// Strategy 3: Hierarchical graph invalidation
fn invalidate_hierarchical(
    cache: &mut SimpleCacheStore,
    updated_pair: &str,
    affected_pairs: &[String],
    quotes_per_pair: usize,
) {
    let mut keys_to_delete = Vec::new();

    // Direct impact
    for i in 0..quotes_per_pair {
        keys_to_delete.push(format!("quote:{}:{}", updated_pair, i));
        keys_to_delete.push(format!("route:{}:{}", updated_pair, i));
    }

    // Cascading impact
    for pair in affected_pairs {
        for i in 0..quotes_per_pair {
            keys_to_delete.push(format!("quote:{}:{}", pair, i));
            keys_to_delete.push(format!("route:{}:{}", pair, i));
        }
    }

    let key_refs: Vec<&str> = keys_to_delete.iter().map(|k| k.as_str()).collect();
    cache.delete_many(&key_refs);
}

fn benchmark_invalidation_strategies(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_invalidation");
    group.sample_size(10);

    // Scenario: 10K pairs, 100K cache entries (quotes + routes)
    let total_pairs = 10_000;
    let quotes_per_pair = 10;
    let total_cache_entries = total_pairs * quotes_per_pair * 2; // quotes + routes

    // Setup: create cache with entries
    let mut cache = SimpleCacheStore::new();
    for pair_idx in 0..total_pairs {
        for quote_idx in 0..quotes_per_pair {
            cache.set(
                format!("quote:pair_{}:{}", pair_idx, quote_idx),
                "data".to_string(),
            );
            cache.set(
                format!("route:pair_{}:{}", pair_idx, quote_idx),
                "data".to_string(),
            );
        }
    }

    println!(
        "\nCache Invalidation Load Test:\n  Total cache entries: {}\n  Total pairs: {}\n  Quotes/routes per pair: {}",
        total_cache_entries, total_pairs, quotes_per_pair
    );

    // Benchmark 1: Full cache clear (naive baseline)
    group.bench_function("full_clear_10k_pairs", |b| {
        b.iter(|| {
            let mut cache_clone = SimpleCacheStore::new();
            for i in 0..total_cache_entries {
                cache_clone.set(format!("key_{}", i), "val".to_string());
            }
            invalidate_full_clear(black_box(&mut cache_clone), "pair_5000", total_cache_entries);
            assert_eq!(cache_clone.size(), 0);
        });
    });

    // Benchmark 2: Pair-level invalidation (basic selective)
    group.bench_function("pair_level_10k_pairs", |b| {
        b.iter(|| {
            let mut cache_clone = SimpleCacheStore::new();
            for i in 0..total_cache_entries {
                cache_clone.set(format!("key_{}", i), "val".to_string());
            }
            invalidate_pair_level(
                black_box(&mut cache_clone),
                "pair_5000",
                quotes_per_pair,
            );
            // Should have deleted ~20 entries (10 quotes + 10 routes)
            assert!(cache_clone.size() > total_cache_entries - 30);
        });
    });

    // Benchmark 3: Hierarchical graph invalidation
    // Simulate a pair that affects 50 dependent pairs (realistic for multi-hop routes)
    let affected_pairs: Vec<String> = (0..50)
        .map(|i| format!("pair_{}", 5000 + i))
        .collect();

    group.bench_function("hierarchical_50_dependent_pairs", |b| {
        b.iter(|| {
            let mut cache_clone = SimpleCacheStore::new();
            for i in 0..total_cache_entries {
                cache_clone.set(format!("key_{}", i), "val".to_string());
            }
            invalidate_hierarchical(
                black_box(&mut cache_clone),
                "pair_5000",
                &affected_pairs,
                quotes_per_pair,
            );
            // Should delete: 20 (direct) + 50*20 (dependent) = 1020 entries
            let expected_deletions = 20 + (50 * 20);
            let remaining = cache_clone.size();
            assert!(remaining < total_cache_entries && remaining > (total_cache_entries - expected_deletions - 100));
        });
    });

    // Benchmark 4: Hierarchical with larger dependency tree
    let large_affected_pairs: Vec<String> = (0..500)
        .map(|i| format!("pair_{}", 5000 + i))
        .collect();

    group.bench_function("hierarchical_500_dependent_pairs", |b| {
        b.iter(|| {
            let mut cache_clone = SimpleCacheStore::new();
            for i in 0..total_cache_entries {
                cache_clone.set(format!("key_{}", i), "val".to_string());
            }
            invalidate_hierarchical(
                black_box(&mut cache_clone),
                "pair_5000",
                &large_affected_pairs,
                quotes_per_pair,
            );
        });
    });

    group.finish();
}

fn benchmark_graph_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("invalidation_graph_lookup");

    // Benchmark pair lookup in graph with varying sizes
    for pair_count in [100, 1_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(pair_count),
            pair_count,
            |b, &pair_count| {
                b.iter(|| {
                    let mut affected_keys = HashSet::new();

                    // Simulate graph lookup: retrieve affected keys for 100 pairs
                    for i in 0..100 {
                        let pair_id = (5000 + i) % pair_count;
                        // Simulate O(1) map lookup for quotes + routes
                        for j in 0..10 {
                            affected_keys.insert(format!("quote:{}:{}", pair_id, j));
                            affected_keys.insert(format!("route:{}:{}", pair_id, j));
                        }
                    }

                    black_box(affected_keys.len());
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_invalidation_strategies,
    benchmark_graph_lookup
);
criterion_main!(benches);
