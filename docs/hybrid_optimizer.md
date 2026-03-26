# Hybrid Route Optimizer

The Hybrid Route Optimizer is a sophisticated routing engine that balances execution quality with computational latency using configurable policies and weighted scoring.

## Overview

The hybrid optimizer addresses the fundamental tradeoff in DEX routing:
- **Quality**: Maximum output amount and minimal price impact
- **Latency**: Fast computation time for real-time trading

By using configurable policies, the optimizer can be tuned for different use cases:
- Production trading (balanced approach)
- Analysis (maximum quality)
- Real-time trading (minimum latency)
- Testing (flexible experimentation)

## Architecture

### Core Components

1. **OptimizerPolicy**: Configuration defining weights and constraints
2. **HybridOptimizer**: Main optimization engine
3. **RouteMetrics**: Comprehensive scoring metrics
4. **OptimizerDiagnostics**: Detailed analysis and alternatives

### Scoring Model

The optimizer uses a weighted scoring model combining three factors:

```
score = output_weight * output_score
      + impact_weight * impact_score  
      + latency_weight * latency_score
```

#### Output Score
- Normalized by target output amount
- Higher values indicate better execution quality

#### Impact Score
- Inverse of price impact (lower impact = higher score)
- Normalized to maximum acceptable impact

#### Latency Score
- Inverse of computation time (faster = higher score)
- Normalized to maximum acceptable time

## Usage

### Basic Usage

```rust
use stellarroute_routing::{HybridOptimizer, PathfinderConfig};

// Create optimizer with default configuration
let optimizer = HybridOptimizer::new(PathfinderConfig::default());

// Find optimal route
let result = optimizer.find_optimal_routes(
    "XLM",           // Source asset
    "USDC",          // Destination asset
    &edges,          // Liquidity edges
    100_000_000,     // Amount (10 XLM in e7 precision)
)?;

println!("Selected route: {:?}", result.selected_path);
println!("Score: {:.4}", result.metrics.score);
println!("Output: {}", result.metrics.output_amount);
println!("Impact: {} bps", result.metrics.impact_bps);
println!("Compute time: {} μs", result.metrics.compute_time_us);
```

### Policy Configuration

#### Using Preset Policies

```rust
use stellarroute_routing::{PolicyPresets, HybridOptimizer};

let mut optimizer = HybridOptimizer::new(PathfinderConfig::default());

// Switch to real-time policy (prioritizes speed)
optimizer.set_active_policy("realtime")?;

// Switch to analysis policy (prioritizes quality)
optimizer.set_active_policy("analysis")?;
```

#### Custom Policies

```rust
use stellarroute_routing::{OptimizerPolicy, HybridOptimizer};

let mut optimizer = HybridOptimizer::new(PathfinderConfig::default());

// Create custom policy
let custom_policy = OptimizerPolicy {
    output_weight: 0.6,    // Prioritize output
    impact_weight: 0.3,     // Consider impact
    latency_weight: 0.1,    // Minimal latency concern
    max_impact_bps: 500,    // 5% max impact
    max_compute_time_ms: 2000, // 2 second timeout
    environment: "custom".to_string(),
};

optimizer.add_policy(custom_policy)?;
optimizer.set_active_policy("custom")?;
```

### Policy Comparison

```rust
// Compare all policies for the same route
let results = optimizer.benchmark_policies("XLM", "USDC", &edges, 100_000_000)?;

for (policy_name, diagnostics) in results {
    println!("Policy: {}", policy_name);
    println!("  Output: {}", diagnostics.metrics.output_amount);
    println!("  Score: {:.4}", diagnostics.metrics.score);
    println!("  Time: {} ms", diagnostics.total_compute_time_ms);
}
```

## Policy Presets

### Production Policy
- **Focus**: Balanced approach for live trading
- **Weights**: Output 50%, Impact 30%, Latency 20%
- **Constraints**: 300 bps max impact, 500ms timeout

### Analysis Policy  
- **Focus**: Maximum execution quality
- **Weights**: Output 70%, Impact 25%, Latency 5%
- **Constraints**: 1000 bps max impact, 5000ms timeout

### Realtime Policy
- **Focus**: Fastest possible execution
- **Weights**: Output 30%, Impact 20%, Latency 50%
- **Constraints**: 500 bps max impact, 100ms timeout

### Testing Policy
- **Focus**: Flexible experimentation
- **Weights**: Output 40%, Impact 30%, Latency 30%
- **Constraints**: 400 bps max impact, 2000ms timeout

## Diagnostics

The optimizer provides comprehensive diagnostics for selected routes:

```rust
let diagnostics = optimizer.find_optimal_routes("XLM", "USDC", &edges, 100_000_000)?;

// Selected route details
println!("Selected route has {} hops", diagnostics.selected_path.hops.len());

// Route metrics
println!("Route score: {:.4}", diagnostics.metrics.score);
println!("Output amount: {}", diagnostics.metrics.output_amount);
println!("Price impact: {} bps", diagnostics.metrics.impact_bps);
println!("Computation time: {} μs", diagnostics.metrics.compute_time_us);

// Alternative routes considered
println!("{} alternative routes evaluated", diagnostics.alternatives.len());
for (i, (path, metrics)) in diagnostics.alternatives.iter().enumerate() {
    println!("  Alt {}: score={:.4}, output={}, impact={} bps", 
             i+1, metrics.score, metrics.output_amount, metrics.impact_bps);
}

// Policy used
println!("Optimized with policy: {}", diagnostics.policy.environment);
```

## Deterministic Behavior

The optimizer guarantees deterministic behavior for identical inputs:
- Same parameters always produce the same results
- Routes are sorted consistently by score
- Alternative routes maintain stable ordering

This ensures reliable testing and reproducible results in production.

## Performance Considerations

### Latency Optimization
- Early termination when constraints are exceeded
- Efficient graph traversal with cycle prevention
- Minimal memory allocations during optimization

### Quality Optimization  
- Comprehensive path exploration within depth limits
- Accurate impact calculations for each hop
- Normalized scoring for fair comparison

### Scalability
- Linear scaling with graph size
- Configurable depth limits prevent explosion
- Efficient data structures for large graphs

## Benchmarks

Run the benchmark suite to evaluate performance:

```bash
# Run all benchmarks
cargo bench -p stellarroute-routing

# Run specific benchmark groups
cargo bench -p stellarroute-routing policy_comparison
cargo bench -p stellarroute-routing latency_vs_quality
cargo bench -p stellarroute-routing scalability
```

### Benchmark Categories

1. **Policy Comparison**: Compare different preset policies
2. **Latency vs Quality**: Tradeoff analysis with time limits
3. **Scalability**: Performance with different graph sizes
4. **Determinism**: Verify consistent results
5. **Policy Benchmarking**: Compare all policies simultaneously

## Integration

### API Integration

The optimizer integrates seamlessly with existing routing APIs:

```rust
use stellarroute_routing::RoutingEngine;

let engine = RoutingEngine::new();

// Use traditional pathfinding
let paths = engine.pathfinder().find_paths("XLM", "USDC", &edges, amount)?;

// Use hybrid optimization
let optimized = engine.hybrid_optimizer().find_optimal_routes("XLM", "USDC", &edges, amount)?;
```

### Configuration Management

Policies can be loaded from configuration files:

```json
{
  "production": {
    "output_weight": 0.5,
    "impact_weight": 0.3,
    "latency_weight": 0.2,
    "max_impact_bps": 300,
    "max_compute_time_ms": 500,
    "environment": "production"
  }
}
```

## Best Practices

### Policy Selection
- Use **production** for live trading
- Use **analysis** for backtesting and research
- Use **realtime** for high-frequency applications
- Use **testing** for development and experimentation

### Performance Tuning
- Adjust `max_depth` in pathfinder config for complexity control
- Set appropriate `max_compute_time_ms` for your latency requirements
- Tune `max_impact_bps` based on your risk tolerance

### Monitoring
- Track `total_compute_time_ms` for performance monitoring
- Monitor `impact_bps` for execution quality
- Watch `score` trends for optimization effectiveness

## Future Enhancements

Planned improvements to the hybrid optimizer:

1. **Machine Learning**: Adaptive policy tuning based on market conditions
2. **Dynamic Weights**: Real-time weight adjustment based on volatility
3. **Multi-Objective**: Additional optimization criteria (gas fees, slippage)
4. **Parallel Processing**: Concurrent path evaluation for improved latency
5. **Caching**: Intelligent result caching for repeated queries

## Troubleshooting

### Common Issues

**No routes found**: Check liquidity constraints and increase `max_impact_bps`
**Slow performance**: Reduce `max_depth` or decrease `max_compute_time_ms`
**Poor quality**: Increase `output_weight` and decrease `latency_weight`

### Debug Information

Enable detailed logging to troubleshoot optimization:

```rust
use tracing::{info, Level};
tracing_subscriber::fmt()
    .with_max_level(Level::DEBUG)
    .init();

// Optimization will now log detailed information
let result = optimizer.find_optimal_routes("XLM", "USDC", &edges, amount)?;
```

This comprehensive hybrid optimizer provides the foundation for intelligent DEX routing that balances quality and performance across diverse trading scenarios.
