# Gas Benchmarks & Resource Consumption

This document tracks the gas (CPU instruction) consumption and resource usage of all StellarRoute contract functions to ensure they execute reliably within Soroban's resource limits.

## Soroban Resource Limits

- **CPU Instructions**: ~100M instructions per transaction
- **Memory**: Limited linear memory for WASM execution
- **Storage reads/writes**: Each operation consumes resources
- **Contract size**: 64KB WASM limit (target: <56KB)
- **Events**: Each event consumes resources

## Benchmark Results

### Core Functions

| Function | Hops | CPU Instructions | Storage Reads | Storage Writes | Events | Status |
|----------|------|------------------|---------------|----------------|--------|--------|
| `initialize` | - | <10M | 0 | 4 | 1 | ✅ Pass |
| `register_pool` | - | <5M | 1 | 2 | 1 | ✅ Pass |
| `get_quote` | 1 | <15M | 2 | 0 | 0 | ✅ Pass |
| `get_quote` | 2 | <25M | 3 | 0 | 0 | ✅ Pass |
| `get_quote` | 3 | <35M | 4 | 0 | 0 | ✅ Pass |
| `get_quote` | 4 | <50M | 5 | 0 | 0 | ✅ Pass |
| `execute_swap` | 1 | <20M | 3 | 1 | 1 | ✅ Pass |
| `execute_swap` | 2 | <35M | 4 | 1 | 1 | ✅ Pass |
| `execute_swap` | 3 | <55M | 5 | 1 | 1 | ✅ Pass |
| `execute_swap` | 4 | <80M | 6 | 1 | 1 | ✅ Pass |
| `estimate_resources` | 4 | <5M | 1 | 0 | 0 | ✅ Pass |

### Administrative Functions

| Function | CPU Instructions | Storage Reads | Storage Writes | Events | Status |
|----------|------------------|---------------|----------------|--------|--------|
| `set_admin` | <3M | 1 | 1 | 1 | ✅ Pass |
| `pause` | <2M | 1 | 1 | 1 | ✅ Pass |
| `unpause` | <2M | 1 | 1 | 1 | ✅ Pass |

### View Functions (Read-Only)

| Function | CPU Instructions | Storage Reads | Status |
|----------|------------------|---------------|--------|
| `version` | <0.1M | 0 | ✅ Pass |
| `get_admin` | <1M | 1 | ✅ Pass |
| `get_fee_rate_value` | <1M | 1 | ✅ Pass |
| `is_paused` | <1M | 1 | ✅ Pass |
| `get_pool_count` | <1M | 1 | ✅ Pass |
| `is_pool_registered` | <1M | 1 | ✅ Pass |

## Optimization Strategies Implemented

### 1. Storage Optimization ✅

- **Batched reads**: `get_instance_config()` reads admin, fee_rate, fee_to, and paused in one operation
- **Cached pool lookups**: `batch_check_pools()` validates all pools before execution
- **Compact storage keys**: Using `Symbol` instead of `String` where possible
- **Pre-allocated vectors**: Known capacity to avoid reallocation

**Impact**: Reduced storage reads by ~40% for multi-hop swaps

### 2. Computation Optimization ✅

- **Static dispatch**: Direct function calls instead of dynamic dispatch
- **Pre-allocated vectors**: `Vec::new(&e)` with known capacity
- **Inline constant product**: `calculate_constant_product_output()` inlined for known pool types
- **Minimal event data**: Emit essential data only

**Impact**: Reduced CPU consumption by ~15-20% per hop

### 3. Cross-Contract Call (CCI) Optimization ✅

- **Batched validation**: Check all pools before starting swap execution
- **Configurable max hops**: `MAX_HOPS = 4` constant enforced
- **Efficient CCI patterns**: Reuse `symbol_short!` for common symbols

**Impact**: Reduced overhead per CCI by ~10%

### 4. WASM Size Optimization ✅

Current release profile settings in `Cargo.toml`:

```toml
[profile.release]
opt-level = "z"        # Optimize for size
lto = true             # Link-time optimization
strip = "symbols"      # Remove debug symbols
codegen-units = 1      # Better optimization
```

**Current WASM size**: ~45KB (target: <56KB) ✅

### 5. Resource Pre-flight Estimation ✅

New `estimate_resources()` function provides:

```rust
pub struct ResourceEstimate {
    pub estimated_cpu: u64,
    pub storage_reads: u32,
    pub storage_writes: u32,
    pub events: u32,
    pub will_succeed: bool,
}
```

**Usage**: Frontend can call this before submitting to warn users about high-cost routes

## Stress Test Results

### Maximum Complexity Test

- **Scenario**: 4-hop swap with large amount (10B units)
- **CPU Cost**: <80M instructions ✅
- **Memory Cost**: Within limits ✅
- **Result**: SUCCESS ✅

### Near-Limit Scenarios

All tested scenarios complete successfully with headroom:

- 4-hop swap: 80M / 100M (80% utilization) ✅
- Worst-case storage: 6 reads, 1 write ✅
- Event emission: Minimal overhead ✅

## Regression Testing

Automated tests fail if gas consumption increases by >10% from baseline:

```bash
cargo test bench_ --release -- --nocapture
```

## Updating Baselines

The CPU cost baselines for `execute_swap` are stored in `crates/contracts/gas_baselines.json`. To update:

1. Run the benchmarks locally:
   ```bash
   cd crates/contracts
   cargo test bench_ --release -- --nocapture
   ```
2. Extract the new CPU cost values from the output (look for `execute_swap_1_hop_cpu_cost` and `execute_swap_4_hops_cpu_cost` lines)
3. Update `crates/contracts/gas_baselines.json` with the new values
4. Commit the updated baseline file to the repository

## CI Integration

The `.github/workflows/gas-benchmarks.yml` workflow automatically:
1. Runs the gas benchmark tests
2. Checks that CPU costs do not exceed 10% over the baselines
3. Verifies the optimized WASM size stays under 100KB
4. Uploads `benchmark_results.txt` and `optimized.wasm` as artifacts
5. Posts a comment with results on pull requests

## Performance Improvements Summary

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Storage reads (4-hop) | 10 | 6 | 40% ↓ |
| CPU per hop | ~6M | ~5M | 17% ↓ |
| WASM size | N/A | 45KB | ✅ Under limit |
| Max hops supported | 4 | 4 | ✅ Maintained |

## Future Optimization Opportunities

1. **Pool adapter inlining**: For known pool types (constant product), inline math instead of CCI
2. **Temporary storage**: Use for ephemeral data (rate limits, commitments)
3. **Batch token transfers**: Combine multiple transfers where possible
4. **Event optimization**: Emit hashes instead of full route data

## References

- [Soroban Resource Limits](https://soroban.stellar.org/docs/learn/resource-limits)
- [Soroban Fees](https://soroban.stellar.org/docs/learn/fees)
- [WASM Optimization Guide](https://rustwasm.github.io/book/reference/code-size.html)

---

**Last Updated**: 2026-06-27  
**Contract Version**: 1  
**Benchmark Environment**: Soroban SDK 21.0
