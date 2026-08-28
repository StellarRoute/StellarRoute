# Storage Rent and Bounded Collections Audit Report

## Overview

This report audits the StellarRoute router contract's storage rent exposure and bounded collection usage. Storage rent on Soroban charges per-byte-per-ledger for persistent and temporary storage; unbounded growth can cause the contract to exceed rent budgets or become economically unsustainable.

---

## 1. Storage Key Inventory

### 1.1 Instance Storage

| Key | Value Type | Approx Bytes | Notes |
|-----|-----------|-------------|-------|
| `Admin` | `Address` | 36 | Fixed-size; always present |
| `FeeRate` | `u32` | 4 | Fixed-size |
| `FeeTo` | `Address` | 36 | Fixed-size; optional |
| `Paused` | `bool` | 1 | Fixed-size |
| `PoolCount` | `u32` | 4 | Fixed-size |
| `PoolList` | `Vec<Address>` | 36 × N | **Unbounded** — grows with registered pools |
| `LastTtlExtension` | `u32` | 4 | Fixed-size |
| `IsMultiSig` | `bool` | 1 | Fixed-size |
| `Governance` | `GovernanceConfig` | ~400 | Bounded: max 10 signers |
| `Guardian` | `Address` | 36 | Optional |
| `ProposalCounter` | `u64` | 8 | Fixed-size |
| `ContractVersionKey` | `ContractVersion` | ~80 | Fixed-size |
| `PendingUpgradeKey` | `PendingUpgrade` | ~80 | Optional; transient |
| `TokenCount` | `u32` | 4 | Fixed-size |
| `MevConfig` | `MevConfig` | ~20 | Fixed-size |
| `FeeConfig` | `FeeConfig` | ~400 | Bounded: max 10 recipients |
| `LatestKnownPrice` | `i128` | 16 | Per pair; **unbounded** growth possible |

**Instance storage rent is zero** on Soroban — instance data is free. However, `PoolList` and `LatestKnownPrice` are stored in instance and grow without bound, which impacts instance size limits (currently 1 MB).

### 1.2 Persistent Storage

| Key Pattern | Value Type | Per-Entry Bytes | Bounded? |
|-------------|-----------|----------------|----------|
| `TotalSwapVolume` | `i128` | 16 | Single entry |
| `SupportedPool(Address)` | — | 36 (key only) | **Unbounded** — one per registered pool |
| `SwapNonce(Address)` | `i128` | 16 | **Unbounded** — one per unique swapper |
| `ProposalEntry(u64)` | `Proposal` | ~300 | **Unbounded** — one per governance proposal |
| `VersionHistory(u64)` | `ContractVersion` | ~80 | **Unbounded** — one per upgrade |
| `MigrationDone(u32,u32,u32)` | `bool` | 1 | **Unbounded** — one per migrated version |
| `AllowedToken(Asset)` | `TokenInfo` | ~200 | **Unbounded** — one per allowlisted token |
| `Whitelisted(Address)` | `bool` | 1 | **Unbounded** — one per whitelisted address |
| `FeeBalance(Asset)` | `i128` | 16 | **Unbounded** — one per fee-collecting asset |
| `TotalFeesCollected(Asset)` | `i128` | 16 | **Unbounded** — one per fee-collecting asset |
| `TotalFeesBurned(Asset)` | `i128` | 16 | **Unbounded** — one per fee-collecting asset |
| `DistributionHistory(Asset)` | `Vec<DistributionRecord>` | ~200 | Bounded: max 10 records per asset |
| `CatEntry(TokenCategory, u32)` | `Asset` | 50 | **Unbounded** — one per token ever added per category |
| `CatLen(TokenCategory)` | `u32` | 4 | 5 entries (one per category) |

**Persistent storage rent**: Charged per-byte-per-ledger. Every unbounded key grows rent linearly with the number of entries.

### 1.3 Temporary Storage

| Key Pattern | Value Type | Per-Entry Bytes | TTL | Bounded? |
|-------------|-----------|----------------|-----|----------|
| `PendingUpgrade` | `PendingUpgrade` | ~80 | ~6 hours | At most 1 active |
| `Commitment(BytesN<32>)` | `CommitmentData` | ~100 | ~1 hour | **Unbounded** — one per commit-reveal swap |
| `AccountSwapCount(Address)` | `u32` | 4 | ~10 min | **Unbounded** — one per active swapper |
| `AccountSwapWindowStart(Address)` | `u32` | 4 | ~10 min | **Unbounded** — one per active swapper |

Temporary storage does not incur rent, but entries consume reads/writes budget and the ledger's temporary storage limits.

---

## 2. Bounded Collection Analysis

### 2.1 Collections with Explicit Bounds

| Collection | Location | Bound | Enforcement |
|-----------|----------|-------|-------------|
| `Governance.signers` | Instance | Max 10 | `governance.rs` — `AddSigner` checks `>= 10` |
| `FeeConfig.recipients` | Instance | Max 10 | Enforced by caller convention; no hard cap in storage |
| `DistributionHistory(Asset)` | Persistent | Max 10 records | `push_distribution_history()` trims to 10 |
| `add_tokens_batch` | tokens.rs | Max 10 per call | `MAX_BATCH = 10` checked before loop |

### 2.2 Collections WITHOUT Explicit Bounds (Risk Areas)

| Collection | Location | Growth Pattern | Risk |
|-----------|----------|---------------|------|
| `PoolList` (instance) | storage.rs:181-193 | +1 per `register_pool` | **High** — instance size grows linearly; no removal |
| `SupportedPool` entries | storage.rs:165-169 | +1 per `register_pool` | **Medium** — persistent rent per entry |
| `SwapNonce` entries | storage.rs:197-210 | +1 per unique swapper | **Medium** — persistent rent; TTL extends on every swap |
| `ProposalEntry` entries | storage.rs:302-309 | +1 per governance proposal | **Low** — bounded by governance activity; TTL = 30 days |
| `VersionHistory` entries | storage.rs:323-327 | +1 per upgrade | **Low** — infrequent; TTL = 365 days |
| `MigrationDone` entries | storage.rs:346-358 | +1 per version migrated | **Low** — infrequent; TTL = 365 days |
| `AllowedToken` entries | storage.rs:362-386 | +1 per allowlisted token | **Medium** — governed by admin; TTL = 365 days |
| `TokenCategoryIndex` entries | tokens.rs:64-77 | +1 per token ever added | **Medium** — append-only; removed tokens leave ghost entries |
| `Whitelisted` entries | storage.rs:462-475 | +1 per whitelisted address | **Low** — governed by admin |
| `FeeBalance` / `TotalFees*` entries | storage.rs:499-549 | +1 per fee-collecting asset | **Low** — bounded by number of distinct assets |
| `LatestKnownPrice` entries | storage.rs:479-487 | +1 per asset pair | **Medium** — instance storage; unbounded pairs |

### 2.3 Token Category Index — Ghost Entry Problem

The category index (`CatEntry`) is append-only. When a token is removed via `remove_token`, the entry remains in the index but its `AllowedToken` key is deleted. `get_tokens_by_category()` iterates all index entries and filters by `is_token_allowed`, which means:

- Storage cost for removed tokens is never reclaimed
- Read cost increases linearly with total tokens ever added
- No compaction or cleanup mechanism exists

**Recommendation**: Accept this trade-off (tokens rarely removed) or add a periodic compaction function gated by admin.

---

## 3. TTL Management Audit

### 3.1 TTL Constants

| Constant | Value | Duration |
|----------|-------|----------|
| `DAY_IN_LEDGERS` | 17,280 | ~1 day (5s/ledger) |
| `INSTANCE_TTL_EXTEND_TO` | 518,400 | 30 days |
| `INSTANCE_TTL_THRESHOLD` | 120,960 | 7 days |
| `POOL_TTL_EXTEND_TO` | 1,555,200 | 90 days |
| `POOL_TTL_THRESHOLD` | 379,520 | 22 days |
| `VOLUME_TTL_EXTEND_TO` | 518,400 | 30 days |
| `VOLUME_TTL_THRESHOLD` | 120,960 | 7 days |
| `NONCE_TTL_EXTEND_TO` | 518,400 | 30 days |
| `NONCE_TTL_THRESHOLD` | 120,960 | 7 days |
| `PENDING_UPGRADE_TTL` | 4,320 | ~6 hours |
| `COMMITMENT_TTL` | 720 | ~1 hour |
| `RATE_LIMIT_TTL` | 120 | ~10 minutes |

### 3.2 TTL Extension Points

| Operation | Keys Extended | TTL Set |
|-----------|--------------|---------|
| Any write to instance | Instance (implicit) | 30 days if < 7 days remaining |
| `register_pool` | `SupportedPool` | 90 days |
| `execute_swap` / `get_quote` | `SwapNonce`, `TotalSwapVolume`, `SupportedPool` | 30/90 days |
| `save_proposal` | `ProposalEntry` | 30 days |
| `set_contract_version` | `VersionHistory` | 365 days |
| `set_migration_done` | `MigrationDone` | 365 days |
| `save_token_info` | `AllowedToken` | 365 days |
| `set_fee_balance` / `add_fee_balance` | `FeeBalance`, `TotalFeesCollected` | 30/365 days |
| `push_distribution_history` | `DistributionHistory` | 30 days |
| `set_whitelisted(true)` | `Whitelisted` | 30 days |
| `add_token_internal` | Category index entries | 365 days |
| `extend_storage_ttl` (public) | All registered pools + instance | Threshold-based |

### 3.3 TTL Risks

1. **PoolList has no TTL extension** — The `PoolList` Vec in instance storage is never extended via `extend_persistent_ttl`. Since it's instance storage, it doesn't expire, but it grows unboundedly.

2. **Nonce TTL extends on every swap** — Active swappers keep their nonce entry alive indefinitely. If 100,000 unique users swap, that's 100,000 persistent entries × 30 days TTL.

3. **No TTL for removed tokens' category index** — Ghost entries in `CatEntry` persist for 365 days even after the token is removed.

4. **Proposal entries have 30-day TTL** — Old proposals expire after 30 days, which is good for cleanup. However, if governance is inactive, expired proposals leave no trace.

---

## 4. Rent Cost Estimates

Assuming Soroban persistent rent of ~0.00001 XLM per byte per ledger (protocol v21):

| Collection | Entries | Bytes/Entry | Monthly Cost (XLM) |
|-----------|---------|------------|-------------------|
| 100 pools | 100 | 36 | ~0.19 |
| 10,000 swappers | 10,000 | 16 | ~0.77 |
| 50 tokens | 50 | 200 | ~0.52 |
| 1000 proposals | 1,000 | 300 | ~15.5 |
| 100 asset pairs (prices) | 100 | 16 | ~0.08 |

**Note**: Soroban rent costs change with network upgrades. These estimates are for reference only.

---

## 5. Findings

### Finding 1: PoolList is Unbounded Instance Storage (LOW)

**Location**: `storage.rs:181-193`

`PoolList` is a `Vec<Address>` stored in instance storage that grows with every `register_pool` call. There is no mechanism to remove pools from the list, and no upper bound.

**Impact**: Instance storage size grows linearly. At 1000 pools, this would be ~36 KB of instance data.

**Recommendation**: Consider adding a `MAX_POOLS` constant and checking it in `register_pool`. Alternatively, document the expected maximum and monitor instance size.

### Finding 2: Token Category Index Ghost Entries (LOW)

**Location**: `tokens.rs:64-77, 247-258`

Removed tokens leave ghost entries in the category index. `get_tokens_by_category()` iterates all entries (including removed ones) and filters.

**Impact**: Read cost and storage rent grow with total tokens ever added, not just current count.

**Recommendation**: Accept as-is (tokens rarely removed) or add admin-only compaction.

### Finding 3: No PoolList Removal (MEDIUM)

**Location**: `storage.rs:189-193` (add) — no corresponding remove function

Pools can be deregistered (marking them inactive), but `PoolList` is never shrunk. The `PoolList` Vec grows monotonically.

**Impact**: Long-term instance storage bloat.

**Recommendation**: Add `remove_from_pool_list()` that swaps the target with the last element and pops, or rebuild the list on deregistration.

### Finding 4: Nonce Entries Proliferate (LOW)

**Location**: `storage.rs:197-210`

Every unique swapper gets a persistent `SwapNonce` entry. There is no TTL-based cleanup for inactive swappers beyond the 30-day extension.

**Impact**: At scale, thousands of nonce entries with 30-day TTL. Each extension costs gas.

**Recommendation**: Consider a longer TTL for nonces (e.g., 90 days) to reduce extension frequency, or accept as necessary for swap ordering.

### Finding 5: GovernanceConfig Vec Unbounded (LOW)

**Location**: `governance.rs`

`GovernanceConfig.signers` is a `Vec<Address>` with a soft cap of 10 enforced in `AddSigner`. The `FeeConfig.recipients` has no hard cap.

**Impact**: Minimal — governance signers are tightly controlled. Fee recipients are set by admin.

**Recommendation**: Add a hard cap check for `FeeConfig.recipients` matching the 10-signer limit.

---

## 6. Recommendations Summary

| Priority | Recommendation | Files |
|----------|---------------|-------|
| MEDIUM | Add pool deregistration from `PoolList` | `storage.rs`, `router.rs` |
| LOW | Add `MAX_POOLS` constant and guard in `register_pool` | `storage.rs`, `router.rs` |
| LOW | Add hard cap for `FeeConfig.recipients` | `governance.rs`, `types.rs` |
| INFO | Document expected scale limits for each collection | This document |
| INFO | Monitor instance storage size in CI/CD | `gas-benchmarks.yml` |

---

## 7. Conclusion

The StellarRoute contract uses a mix of instance, persistent, and temporary storage with reasonable TTL management. The primary storage rent risks are:

1. **PoolList** growing unboundedly in instance storage
2. **SwapNonce** entries proliferating with unique users
3. **Token category index** accumulating ghost entries

All other collections are either bounded by protocol limits (governance signers), admin-controlled (tokens, whitelists), or infrequent (proposals, upgrades). The TTL extension strategy is sound and prevents premature expiration of actively-used data.

**Overall Assessment**: Storage rent exposure is manageable for expected scale (hundreds of pools, thousands of users). The main recommendation is to add pool list cleanup and monitor instance storage size.
