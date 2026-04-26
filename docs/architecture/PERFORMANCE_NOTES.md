# Route Computation Performance Notes

## Optimization Targets

Achieved performance metrics for multi-hop route discovery:

### Pathfinding Performance
- **2-hop routes**: < 1ms (SDEX→AMM pattern)
- **4-hop routes**: < 5ms on realistic graph sizes (100K+ nodes)
- **Max depth**: Configurable (default: 4 hops)

### Hot Path Optimizations
1. **Cycle Prevention**: O(n) visited set tracking per BFS node
2. **Liquidity Threshold Filtering**: Pre-filters low-liquidity edges during graph construction
3. **BFS Early Termination**: Stops exploring after max_depth reached
4. **Graph Adjacency Caching**: Builds once, reused for all path queries

### Price Impact Calculation
- **Orderbook Impact**: Partial fill processing - O(n) where n = orderbook depth
- **AMM Constant Product**: Single calculation - O(1) with overflow safety
- **Precision**: 1e7 scale (10 decimals) for high-precision arithmetic

### Benchmark Suite
Run benchmarks with:
```bash
cargo bench -p stellarroute-routing --bench routing_benchmarks
```

Key benchmarks:
- `pathfind_2hop`: 2-hop discovery baseline
- `pathfind_4hop_realistic`: Full depth with realistic graph connectivity
- `amm_quote_constant_product`: Single AMM quote
- `amm_quote_large_trade_4M_reserve`: Impact on large trades

## Quote Serialization

The `/api/v1/quote` hot path now caches the fully serialized response body and
reuses it on cache hits. This avoids the previous `QuoteResponse -> JSON`
serialization work on every cached response while preserving the exact wire
contract.

Run the API serialization benchmark with:
```bash
cargo bench -p stellarroute-api --bench quote_serialization
```

Key benchmarks:
- `quote_response_serialize_each_time`: Baseline cost of serializing a representative quote payload.
- `quote_response_cached_json_reuse`: Optimized cache-hit path that reuses prebuilt JSON bytes.

For p95 validation in an environment with Redis enabled, compare
`histogram_quantile(0.95, rate(stellarroute_quote_request_duration_seconds_bucket[5m]))`
before and after deployment while driving repeated identical quote requests to
exercise the cache-hit path.

## Safety Bounds

### Route Discovery
- Max depth: 4 (configurable)
- Min liquidity threshold: 1M units (e7 scale)
- Cycle prevention: Complete visited set tracking
- Graph size: Tested with 50K+ edges

### Price Impact
- Overflow protection on all multiplication operations
- Precision validation for e7-scale calculations
- Trade size validation: max 50% of reserve
- Protected against division by zero and negative reserves

## Future Optimizations
- Memoization of frequently-used paths
- Parallel path discovery on large graphs
- Approximate nearest-neighbor for intermediate asset selection
