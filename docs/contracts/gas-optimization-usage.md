# Gas Optimization Features - Usage Guide

This guide explains how to use the new gas optimization features implemented in issue #62.

## For Frontend Developers

### Resource Estimation Before Swap

Before submitting a swap transaction, you can estimate its resource consumption:

```typescript
import { StellarRouteClient } from './stellar-route-client';

// Create route with multiple hops
const route = {
  hops: [
    { source: assetA, destination: assetB, pool: poolAddress1, pool_type: 'AmmConstProd' },
    { source: assetB, destination: assetC, pool: poolAddress2, pool_type: 'AmmConstProd' },
    { source: assetC, destination: assetD, pool: poolAddress3, pool_type: 'AmmConstProd' },
  ],
  estimated_output: 0,
  min_output: 0,
  expires_at: 0,
};

// Estimate resources
const estimate = await client.estimate_resources({
  amount_in: 1_000_000,
  route: route,
});

console.log('Estimated CPU:', estimate.estimated_cpu);
console.log('Storage reads:', estimate.storage_reads);
console.log('Storage writes:', estimate.storage_writes);
console.log('Events:', estimate.events);
console.log('Will succeed:', estimate.will_succeed);

// Warn user if transaction might fail
if (!estimate.will_succeed) {
  alert('This route is too complex and may exceed gas limits. Try a simpler route.');
  return;
}

// Warn user about high gas costs
if (estimate.estimated_cpu > 50_000_000) {
  const confirmed = confirm(
    `This swap will consume ${estimate.estimated_cpu / 1_000_000}M CPU instructions. ` +
    'It may be expensive. Continue?'
  );
  if (!confirmed) return;
}

// Proceed with swap
await client.execute_swap({
  sender: userAddress,
  params: swapParams,
});
```

### Optimal Route Selection

When you have multiple possible routes, choose the one with lowest gas consumption:

```typescript
async function findOptimalRoute(routes: Route[]): Promise<Route> {
  const estimates = await Promise.all(
    routes.map(route => client.estimate_resources({ amount_in: 1_000_000, route }))
  );
  
  // Filter out routes that won't succeed
  const validRoutes = routes.filter((_, i) => estimates[i].will_succeed);
  
  if (validRoutes.length === 0) {
    throw new Error('No valid routes found');
  }
  
  // Find route with lowest CPU cost
  let bestIndex = 0;
  let lowestCpu = estimates[0].estimated_cpu;
  
  for (let i = 1; i < validRoutes.length; i++) {
    if (estimates[i].estimated_cpu < lowestCpu) {
      lowestCpu = estimates[i].estimated_cpu;
      bestIndex = i;
    }
  }
  
  return validRoutes[bestIndex];
}
```

## For Contract Developers

### Batched Storage Reads

The contract now uses batched storage reads for better performance:

```rust
// Old way (multiple storage reads)
let admin = storage::get_admin(&e);
let fee_rate = storage::get_fee_rate(&e);
let fee_to = storage::get_fee_to(&e);
let paused = storage::get_paused(&e);

// New way (single batched read)
let config = storage::get_instance_config(&e);
// config.admin, config.fee_rate, config.fee_to, config.paused
```

### Batched Pool Validation

Validate all pools at once instead of one by one:

```rust
// Old way (N storage reads)
for hop in route.hops.iter() {
    if !storage::is_supported_pool(&e, hop.pool.clone()) {
        return Err(ContractError::PoolNotSupported);
    }
}

// New way (batched validation)
let mut pools = Vec::new(&e);
for hop in route.hops.iter() {
    pools.push_back(hop.pool.clone());
}
if !storage::batch_check_pools(&e, &pools) {
    return Err(ContractError::PoolNotSupported);
}
```

### Inline Constant Product Calculation

For known pool types, use inline calculation instead of cross-contract calls:

```rust
use crate::storage::calculate_constant_product_output;

// Instead of CCI to pool
let output = calculate_constant_product_output(
    reserve_in,
    reserve_out,
    amount_in,
);
```

## For DevOps / CI

### Running Benchmarks Locally

```bash
# Run all benchmark tests
cd crates/contracts
cargo test bench_ --lib -- --nocapture

# Run specific benchmark
cargo test bench_execute_swap_4_hops --lib -- --nocapture

# Run stress tests
cargo test stress_test --lib -- --nocapture

# Run regression tests
cargo test regression_test --lib -- --nocapture
```

### CI Integration

The gas benchmarks workflow runs automatically on:
- Push to `main` or `develop` branches
- Pull requests targeting `main` or `develop`
- Changes to `crates/contracts/**`

It will:
1. Run all benchmark tests
2. Check WASM size (<56KB)
3. Optimize WASM with wasm-opt
4. Comment PR with results
5. Fail if thresholds exceeded

### Monitoring WASM Size

```bash
# Build release WASM
cargo build --release --target wasm32-unknown-unknown

# Check size
ls -lh target/wasm32-unknown-unknown/release/*.wasm

# Optimize with wasm-opt
wasm-opt -Oz target/wasm32-unknown-unknown/release/stellarroute_contracts.wasm \
  -o optimized.wasm

# Check optimized size
ls -lh optimized.wasm
```

## Performance Metrics

### Expected CPU Costs

| Operation | Hops | Expected CPU | Max Allowed |
|-----------|------|--------------|-------------|
| get_quote | 1 | ~10M | 15M |
| get_quote | 2 | ~18M | 25M |
| get_quote | 4 | ~35M | 50M |
| execute_swap | 1 | ~15M | 20M |
| execute_swap | 2 | ~28M | 35M |
| execute_swap | 4 | ~60M | 80M |

### Storage Operations

| Operation | Reads | Writes |
|-----------|-------|--------|
| initialize | 0 | 4 |
| register_pool | 1 | 2 |
| get_quote (4 hops) | 5 | 0 |
| execute_swap (4 hops) | 6 | 1 |

## Troubleshooting

### Transaction Fails with "Budget Exceeded"

1. Check number of hops - reduce if >4
2. Use `estimate_resources()` to check before submitting
3. Consider splitting into multiple transactions
4. Check if pools are registered (unregistered pools cause extra overhead)

### WASM Size Exceeds Limit

1. Review dependencies - remove unused features
2. Check for duplicate dependencies in Cargo.lock
3. Use `cargo tree` to analyze dependency tree
4. Consider splitting into multiple contracts

### Benchmark Tests Failing

1. Check if code changes increased gas consumption
2. Review optimization strategies in gas-benchmarks.md
3. Profile specific functions causing increases
4. Consider reverting changes or optimizing further

## Best Practices

1. **Always estimate resources** before submitting complex swaps
2. **Prefer fewer hops** when possible (1-2 hops optimal)
3. **Batch operations** when making multiple storage reads
4. **Pre-allocate vectors** with known capacity
5. **Use inline calculations** for known pool types
6. **Monitor WASM size** in CI - keep under 50KB for headroom
7. **Run benchmarks** before merging optimization changes
8. **Document gas costs** for new features

## References

- [Gas Benchmarks Documentation](./gas-benchmarks.md)
- [Soroban Resource Limits](https://soroban.stellar.org/docs/learn/resource-limits)
- [Soroban Fees](https://soroban.stellar.org/docs/learn/fees)
- [Implementation Summary](../../IMPLEMENTATION_SUMMARY.md)
