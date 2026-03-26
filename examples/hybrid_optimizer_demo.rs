//! Hybrid Route Optimizer Demo
//! 
//! This example demonstrates the hybrid optimizer's capabilities including:
//! - Policy comparison and switching
//! - Route optimization with different constraints
//! - Performance benchmarking
//! - Diagnostics and analysis

use stellarroute_routing::{
    HybridOptimizer, LiquidityEdge, OptimizerPolicy, PolicyPresets, 
    PathfinderConfig, RouteMetrics
};
use std::collections::HashMap;
use std::time::Instant;

fn create_sample_liquidity_graph() -> Vec<LiquidityEdge> {
    vec![
        // XLM liquidity pools
        LiquidityEdge {
            from: "XLM".to_string(),
            to: "USDC".to_string(),
            venue_type: "amm".to_string(),
            venue_ref: "pool_xlm_usdc".to_string(),
            liquidity: 2_000_000_000, // 200 XLM
        },
        LiquidityEdge {
            from: "XLM".to_string(),
            to: "EURT".to_string(),
            venue_type: "orderbook".to_string(),
            venue_ref: "book_xlm_eurt".to_string(),
            liquidity: 1_500_000_000, // 150 XLM
        },
        LiquidityEdge {
            from: "XLM".to_string(),
            to: "BTC".to_string(),
            venue_type: "amm".to_string(),
            venue_ref: "pool_xlm_btc".to_string(),
            liquidity: 500_000_000, // 50 XLM
        },
        
        // USDC liquidity pools
        LiquidityEdge {
            from: "USDC".to_string(),
            to: "EURT".to_string(),
            venue_type: "amm".to_string(),
            venue_ref: "pool_usdc_eurt".to_string(),
            liquidity: 1_800_000_000, // 180 USDC
        },
        LiquidityEdge {
            from: "USDC".to_string(),
            to: "BTC".to_string(),
            venue_type: "orderbook".to_string(),
            venue_ref: "book_usdc_btc".to_string(),
            liquidity: 800_000_000, // 80 USDC
        },
        LiquidityEdge {
            from: "USDC".to_string(),
            to: "ETH".to_string(),
            venue_type: "amm".to_string(),
            venue_ref: "pool_usdc_eth".to_string(),
            liquidity: 1_200_000_000, // 120 USDC
        },
        
        // EURT liquidity pools
        LiquidityEdge {
            from: "EURT".to_string(),
            to: "BTC".to_string(),
            venue_type: "amm".to_string(),
            venue_ref: "pool_eurt_btc".to_string(),
            liquidity: 600_000_000, // 60 EURT
        },
        LiquidityEdge {
            from: "EURT".to_string(),
            to: "ETH".to_string(),
            venue_type: "orderbook".to_string(),
            venue_ref: "book_eurt_eth".to_string(),
            liquidity: 400_000_000, // 40 EURT
        },
        
        // BTC liquidity pools
        LiquidityEdge {
            from: "BTC".to_string(),
            to: "ETH".to_string(),
            venue_type: "amm".to_string(),
            venue_ref: "pool_btc_eth".to_string(),
            liquidity: 300_000_000, // 30 BTC
        },
        
        // Additional paths for complexity
        LiquidityEdge {
            from: "ETH".to_string(),
            to: "USDT".to_string(),
            venue_type: "orderbook".to_string(),
            venue_ref: "book_eth_usdt".to_string(),
            liquidity: 900_000_000, // 90 ETH
        },
        LiquidityEdge {
            from: "USDT".to_string(),
            to: "USDC".to_string(),
            venue_type: "amm".to_string(),
            venue_ref: "pool_usdt_usdc".to_string(),
            liquidity: 3_000_000_000, // 300 USDT
        },
    ]
}

fn demo_basic_optimization() {
    println!("=== Basic Optimization Demo ===");
    
    let edges = create_sample_liquidity_graph();
    let optimizer = HybridOptimizer::new(PathfinderConfig::default());
    
    let trade_amount = 100_000_000; // 10 XLM
    
    println!("Finding optimal route for {} XLM -> BTC", trade_amount / 10_000_000);
    
    let start_time = Instant::now();
    match optimizer.find_optimal_routes("XLM", "BTC", &edges, trade_amount) {
        Ok(diagnostics) => {
            let total_time = start_time.elapsed();
            
            println!("✅ Route found in {} ms", total_time.as_millis());
            println!("📍 Path: {} hops", diagnostics.selected_path.hops.len());
            
            for (i, hop) in diagnostics.selected_path.hops.iter().enumerate() {
                println!("  {}. {} -> {} ({})", 
                         i + 1, hop.source_asset, hop.destination_asset, hop.venue_type);
            }
            
            println!("📊 Metrics:");
            println!("  Output amount: {}", diagnostics.metrics.output_amount);
            println!("  Price impact: {} bps", diagnostics.metrics.impact_bps);
            println!("  Compute time: {} μs", diagnostics.metrics.compute_time_us);
            println!("  Score: {:.4}", diagnostics.metrics.score);
            println!("  Alternatives considered: {}", diagnostics.alternatives.len());
        }
        Err(e) => {
            println!("❌ No route found: {}", e);
        }
    }
    println!();
}

fn demo_policy_comparison() {
    println!("=== Policy Comparison Demo ===");
    
    let edges = create_sample_liquidity_graph();
    let mut optimizer = HybridOptimizer::new(PathfinderConfig::default());
    
    let trade_amount = 50_000_000; // 5 XLM
    let from_asset = "XLM";
    let to_asset = "ETH";
    
    println!("Comparing policies for {} {} -> {}", trade_amount / 10_000_000, from_asset, to_asset);
    
    let policies = vec![
        ("Production", PolicyPresets::production()),
        ("Analysis", PolicyPresets::analysis()),
        ("Realtime", PolicyPresets::realtime()),
        ("Testing", PolicyPresets::testing()),
    ];
    
    let mut results = HashMap::new();
    
    for (name, policy) in policies {
        let policy_name = policy.environment.clone();
        optimizer.add_policy(policy).unwrap();
        optimizer.set_active_policy(&policy_name).unwrap();
        
        let start_time = Instant::now();
        if let Ok(diagnostics) = optimizer.find_optimal_routes(from_asset, to_asset, &edges, trade_amount) {
            let query_time = start_time.elapsed();
            
            results.insert(name, (diagnostics, query_time));
            
            println!("✅ {}: Score {:.4}, Output {}, Impact {} bps, Time {} ms",
                     name,
                     diagnostics.metrics.score,
                     diagnostics.metrics.output_amount,
                     diagnostics.metrics.impact_bps,
                     query_time.as_millis());
        } else {
            println!("❌ {}: No route found", name);
        }
    }
    
    // Analysis
    println!("\n📈 Policy Analysis:");
    if let Some((analysis_diag, _)) = results.get("Analysis") {
        if let Some((realtime_diag, _)) = results.get("Realtime") {
            let output_diff = analysis_diag.metrics.output_amount as f64 - realtime_diag.metrics.output_amount as f64;
            let output_improvement = (output_diff / realtime_diag.metrics.output_amount as f64) * 100.0;
            
            println!("  Analysis vs Realtime:");
            println!("    Output improvement: {:.2}%", output_improvement);
            println!("    Analysis hops: {}", analysis_diag.selected_path.hops.len());
            println!("    Realtime hops: {}", realtime_diag.selected_path.hops.len());
        }
    }
    println!();
}

fn demo_custom_policy() {
    println!("=== Custom Policy Demo ===");
    
    let edges = create_sample_liquidity_graph();
    let mut optimizer = HybridOptimizer::new(PathfinderConfig::default());
    
    // Create custom policies with different priorities
    let custom_policies = vec![
        ("Quality First", OptimizerPolicy {
            output_weight: 0.8,
            impact_weight: 0.15,
            latency_weight: 0.05,
            max_impact_bps: 800,
            max_compute_time_ms: 3000,
            environment: "quality_first".to_string(),
        }),
        ("Speed First", OptimizerPolicy {
            output_weight: 0.2,
            impact_weight: 0.1,
            latency_weight: 0.7,
            max_impact_bps: 600,
            max_compute_time_ms: 50,
            environment: "speed_first".to_string(),
        }),
        ("Balanced", OptimizerPolicy {
            output_weight: 0.4,
            impact_weight: 0.35,
            latency_weight: 0.25,
            max_impact_bps: 400,
            max_compute_time_ms: 1000,
            environment: "balanced".to_string(),
        }),
    ];
    
    let trade_amount = 75_000_000; // 7.5 XLM
    
    for (name, policy) in custom_policies {
        let env_name = policy.environment.clone();
        optimizer.add_policy(policy).unwrap();
        optimizer.set_active_policy(&env_name).unwrap();
        
        println!("Testing '{}' policy:", name);
        
        match optimizer.find_optimal_routes("XLM", "USDC", &edges, trade_amount) {
            Ok(diagnostics) => {
                println!("  ✅ Score: {:.4}", diagnostics.metrics.score);
                println!("  📤 Output: {}", diagnostics.metrics.output_amount);
                println!("  💥 Impact: {} bps", diagnostics.metrics.impact_bps);
                println!("  ⏱️  Time: {} μs", diagnostics.metrics.compute_time_us);
                println!("  🔀 Hops: {}", diagnostics.selected_path.hops.len());
            }
            Err(e) => {
                println!("  ❌ No route: {}", e);
            }
        }
        println!();
    }
}

fn demo_benchmark_all() {
    println!("=== Full Policy Benchmark Demo ===");
    
    let edges = create_sample_liquidity_graph();
    let mut optimizer = HybridOptimizer::new(PathfinderConfig::default());
    
    let trade_amount = 25_000_000; // 2.5 XLM
    
    println!("Benchmarking all policies for {} XLM -> USDT", trade_amount / 10_000_000);
    
    let start_time = Instant::now();
    match optimizer.benchmark_policies("XLM", "USDT", &edges, trade_amount) {
        Ok(results) => {
            let benchmark_time = start_time.elapsed();
            
            println!("✅ Benchmark completed in {} ms", benchmark_time.as_millis());
            println!("📊 Results:");
            
            // Sort by score for comparison
            let mut sorted_results: Vec<_> = results.iter().collect();
            sorted_results.sort_by(|a, b| b.1.metrics.score.partial_cmp(&a.1.metrics.score).unwrap());
            
            for (i, (policy_name, diagnostics)) in sorted_results.iter().enumerate() {
                println!("  {}. {} (Score: {:.4})",
                         i + 1, policy_name, diagnostics.metrics.score);
                println!("     Output: {}, Impact: {} bps, Time: {} μs",
                         diagnostics.metrics.output_amount,
                         diagnostics.metrics.impact_bps,
                         diagnostics.metrics.compute_time_us);
                println!("     Hops: {}, Alternatives: {}",
                         diagnostics.selected_path.hops.len(),
                         diagnostics.alternatives.len());
            }
        }
        Err(e) => {
            println!("❌ Benchmark failed: {}", e);
        }
    }
    println!();
}

fn demo_determinism() {
    println!("=== Determinism Demo ===");
    
    let edges = create_sample_liquidity_graph();
    let optimizer = HybridOptimizer::new(PathfinderConfig::default());
    
    let trade_amount = 30_000_000; // 3 XLM
    
    println!("Testing deterministic behavior with identical inputs...");
    
    let mut results = Vec::new();
    
    // Run the same query multiple times
    for i in 1..=5 {
        match optimizer.find_optimal_routes("EURT", "BTC", &edges, trade_amount) {
            Ok(diagnostics) => {
                results.push(diagnostics.metrics.clone());
                println!("  Run {}: Score {:.4}, Output {}, Impact {} bps",
                         i, diagnostics.metrics.score, diagnostics.metrics.output_amount, diagnostics.metrics.impact_bps);
            }
            Err(e) => {
                println!("  Run {}: Error {}", i, e);
            }
        }
    }
    
    // Verify all results are identical
    if results.len() > 1 {
        let first = &results[0];
        let all_identical = results.iter().all(|r| {
            r.output_amount == first.output_amount &&
            r.impact_bps == first.impact_bps &&
            (r.score - first.score).abs() < f64::EPSILON
        });
        
        if all_identical {
            println!("✅ All results are identical - deterministic behavior confirmed!");
        } else {
            println!("❌ Results differ - non-deterministic behavior detected!");
        }
    }
    println!();
}

fn demo_route_analysis() {
    println!("=== Route Analysis Demo ===");
    
    let edges = create_sample_liquidity_graph();
    let optimizer = HybridOptimizer::new(PathfinderConfig::default());
    
    let trade_amount = 80_000_000; // 8 XLM
    
    println!("Analyzing route options for {} XLM -> ETH", trade_amount / 10_000_000);
    
    match optimizer.find_optimal_routes("XLM", "ETH", &edges, trade_amount) {
        Ok(diagnostics) => {
            println!("🏆 Selected Route:");
            print_route_details(&diagnostics.selected_path, &diagnostics.metrics);
            
            if !diagnostics.alternatives.is_empty() {
                println!("\n🔄 Alternative Routes:");
                for (i, (alt_path, alt_metrics)) in diagnostics.alternatives.iter().enumerate().take(3) {
                    println!("  Alternative {} (Score: {:.4}):", i + 1, alt_metrics.score);
                    print_route_details(alt_path, alt_metrics);
                }
            }
            
            println!("\n📈 Optimization Summary:");
            println!("  Policy: {}", diagnostics.policy.environment);
            println!("  Total compute time: {} ms", diagnostics.total_compute_time_ms);
            println!("  Routes evaluated: {}", diagnostics.alternatives.len() + 1);
            
            // Policy breakdown
            let policy = &diagnostics.policy;
            println!("  Policy weights: Output {:.1}%, Impact {:.1}%, Latency {:.1}%",
                     policy.output_weight * 100.0,
                     policy.impact_weight * 100.0,
                     policy.latency_weight * 100.0);
        }
        Err(e) => {
            println!("❌ No route found: {}", e);
        }
    }
    println!();
}

fn print_route_details(path: &stellarroute_routing::SwapPath, metrics: &RouteMetrics) {
    for (i, hop) in path.hops.iter().enumerate() {
        println!("    {}. {} -> {} ({})", i + 1, hop.source_asset, hop.destination_asset, hop.venue_type);
    }
    println!("    📤 Output: {}, 💥 Impact: {} bps, ⏱️ Time: {} μs",
             metrics.output_amount, metrics.impact_bps, metrics.compute_time_us);
}

fn main() {
    println!("🚀 StellarRoute Hybrid Route Optimizer Demo\n");
    
    demo_basic_optimization();
    demo_policy_comparison();
    demo_custom_policy();
    demo_benchmark_all();
    demo_determinism();
    demo_route_analysis();
    
    println!("✨ Demo completed! The hybrid optimizer demonstrates:");
    println!("  ✅ Intelligent route optimization with configurable policies");
    println!("  ✅ Balance between execution quality and computational latency");
    println!("  ✅ Deterministic behavior for reliable results");
    println!("  ✅ Comprehensive diagnostics and analysis");
    println!("  ✅ Flexible policy system for different use cases");
}
