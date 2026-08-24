# Canonical Asset-Pair Ordering

StellarRoute enforces a **consistent canonical ordering** for asset pairs across
API endpoints, cache keys, and the internal routing graph. This guarantees that
the same trading pair always produces the same identifier regardless of the
order in which the two assets are supplied.

---

## Normalization rules

### 1. Asset normalization

Each asset identifier is first normalised individually:

| Input | Canonical form |
|-------|----------------|
| `"XLM"`, `"xlm"`, `"native"` | `"native"` |
| `"USDC"`, `"usdc"` | `"USDC"` |
| `"usdc:GA5ZSEJ..."`, `"USDC:GA5ZSEJ..."` | `"USDC:GA5ZSEJ..."` |

Issuer suffixes are uppercased together with the code.

### 2. Pair ordering

After individual normalisation the two canonical strings are compared
**lexicographically** (byte-by-byte ASCII). The lexicographically smaller string
becomes `base` and the larger becomes `quote`.

```
native          → "native"
USDC            → "USDC"
USDC:GA5ZSEJ…   → "USDC:GA5ZSEJ…"
BTC             → "BTC"
```

**Lexicographic order** (ASCII code-point order):

```
'B' (66) < 'U' (85) < 'n' (110)
BTC  <  USDC  <  native
```

### Examples

| Supplied as `:base` | Supplied as `:quote` | Canonical `(base, quote)` |
|---------------------|----------------------|---------------------------|
| `XLM`               | `USDC`               | `(USDC, native)`          |
| `USDC`              | `XLM`                | `(USDC, native)`          |
| `native`            | `BTC:GB…`            | `(BTC:GB…, native)`       |
| `USDC:GA5…`         | `USDC`               | `(USDC, USDC:GA5…)`       |
| `XLM`               | `xlm`                | `(native, native)`        |

---

## Regression tests

The ordering rules are pinned by unit tests in
`crates/api/src/routes/pairs.rs` (module `routes::pairs::tests`).
Both `GET /api/v1/pairs` and `GET /api/v1/markets` share a single code
path via `list_pairs`, so the tests cover both endpoints simultaneously.

| Test | What it guards |
|---|---|
| `sort_is_ascending_by_canonical_base_then_counter` | Final list is ascending by `(base_asset, counter_asset)` |
| `sort_is_idempotent` | Sorting an already-sorted list is a no-op |
| `sort_empty_list_is_noop` | Empty input does not panic |
| `sort_single_element_unchanged` | One-element list is unchanged |
| `sort_tie_breaks_on_counter_asset` | Tie on `base_asset` breaks on `counter_asset` ascending |
| `normalize_infos_selling_becomes_base_when_lex_smaller` | Selling asset becomes `base` when lexicographically smaller |
| `normalize_infos_buying_becomes_base_when_lex_smaller` | Buying asset becomes `base` when lexicographically smaller |
| `normalize_infos_is_commutative` | Per-pair normalization is independent of input order |
| `markets_and_pairs_apply_identical_sort_and_normalization` | `/markets` and `/pairs` produce identical canonical ordering |
| `asset_canonical_form_native` | `AssetInfo::native()` → `"native"` / display `"XLM"` |
| `asset_canonical_form_credit4_with_issuer` | Credit4 with issuer → `"CODE:ISSUER"` |
| `asset_canonical_form_credit12_with_issuer` | 12-char codes formatted correctly |
| `canonical_ascii_ordering_btc_lt_usdc_lt_native` | ASCII ordering: `BTC:… < USDC:… < native` |

Run with:

```sh
cargo test -p stellarroute-api routes::pairs::tests
```

---

## Where canonical ordering is applied

| Component | Scope | Function |
|-----------|-------|----------|
| **Routing crate** | `stellarroute_routing` | `normalize_asset()`, `normalize_pair()`, `normalize_pair_owned()` |
| **API cache keys** | `orderbook`, `liquidity_revision`, `quote_pair_pattern` | Keys are constructed with canonical ordering |
| **Pairs listing** | `GET /api/v1/pairs` | Response sorted by canonical `base_asset` then `counter_asset` |
| **Liquidity alerts** | `pair_key()` | Threshold lookup keys use canonical ordering |
| **Exactly-once ledger** | `RequestIdentity::canonical_key()` | Individual assets are normalised (pair order preserved for direction semantics) |

### Notable exceptions

The **quote cache key** (`v2:quote:{base}:{quote}:…`) does **not** swap
base/quote order because the base/quote order together with `quote_type`
encodes the trade direction. The **exactly-once deduplication key** similarly
preserves the original pair order for the same reason.

---

## Cache key migration (legacy inverted keys)

Before this standard was enforced, cache keys were built with the raw
user-supplied asset order.  Deployments with existing Redis data may contain
keys in the **old, unnormalised format** for the same trading pair, e.g.:

| Trading pair | Old key (v1) | New key (v2) |
|--------------|--------------|--------------|
| XLM→USDC     | `orderbook:XLM:USDC` | `orderbook:native:USDC` |
| USDC→XLM     | `orderbook:USDC:XLM` | `orderbook:native:USDC` |

Both old and new keys will be served during the migration window because the
canonical key functions always return the new form; old keys will expire
naturally via their TTL.  No explicit migration script is required, but
operators should **monitor the cache hit ratio** after deploying.

Service restarts will flush the entire Redis cache when the `REDIS_URL`
environment variable is unset (i.e. cache is disabled).

---

## Reference implementation

The canonical ordering logic lives in the shared routing crate:

- `crates/routing/src/lib.rs` — `normalize_asset()`, `normalize_pair()`,
  `normalize_pair_owned()`

All API and indexer crates import from this single source of truth.
