# Quote Inspector Operator Guide

## 1. Overview

The `/api/v1/quote` endpoint is the core of StellarRoute's swap experience: it accepts a sell or buy intent and returns the best executable price across SDEX order books and AMM liquidity pools. This runbook gives operators and integration engineers a definitive reference for every field the quote pipeline produces, explains the constraints on the prepare → sign → submit flow, and documents current AMM limitations so on-call engineers can diagnose failures quickly. It does not replace the live schema — see the [Swagger UI](/api-docs) for the authoritative OpenAPI definition and to try requests interactively.

---

## 2. QuoteResponse Field Reference

### 2.1 UI → API Field Mapping

The table below maps every label visible on the `/swap` screen to its corresponding OpenAPI field in `QuoteResponse`.

| UI Label | OpenAPI Field | Type | Notes |
|---|---|---|---|
| "You Pay" amount | `amount` | string | The sell input amount |
| "You Receive" amount | `total` | string | `amount × price` |
| Exchange rate | `price` | string | Per-unit price |
| Quote type badge | `quote_type` | string | `"sell"` or `"buy"` |
| From asset | `base_asset` | AssetInfo | See AssetInfo shape below |
| To asset | `quote_asset` | AssetInfo | See AssetInfo shape below |
| Route path | `path[]` | PathStep[] | See section 2.2 |
| Quote expires | `expires_at` | int (ms) | Optional; Unix ms; null when TTL not set |
| Data freshness | `data_freshness` | DataFreshness | Optional; `fresh_count`, `stale_count`, `max_staleness_secs` |
| Degraded badge | `degraded` | boolean | `true` when market data is soft-stale; default `false` |
| Venue | `path[].source` | string | `"sdex"` or `"amm:{pool_address}"` |
| Price impact | `price_impact` | string | Optional; decimal percentage |
| Midpoint / spread | `midpoint`, `spread_bps` | string / int | Optional; market reference fields |

**AssetInfo shape**

Each `AssetInfo` object has three fields:

| Field | Type | Notes |
|---|---|---|
| `asset_type` | string | `"native"` for XLM; `"credit_alphanum4"` or `"credit_alphanum12"` for issued assets |
| `asset_code` | string \| null | Null for native XLM |
| `asset_issuer` | string \| null | Null for native XLM; Stellar account ID (G…) for issued assets |

---

### 2.2 PathStep Sub-fields

Each element of `path[]` is a `PathStep` describing one hop in the route.

| Field | Type | Optional | Notes |
|---|---|---|---|
| `from_asset` | AssetInfo | no | Asset being sold in this hop |
| `to_asset` | AssetInfo | no | Asset being bought in this hop |
| `price` | string | no | Exchange rate for this hop |
| `source` | string | no | `"sdex"` or `"amm:<pool_address>"` — see section 3 |
| `liquidity_depth` | string | yes | Available depth at the quoted price level |
| `fee_bps` | int | yes | Fee applied to this hop in basis points (e.g. `30` = 0.30%) |

---

### 2.3 Rationale (Explain Mode)

The `rationale` field is only present when the request includes the `X-Explain: true` header (or the `?explain=true` query parameter). It provides venue-selection metadata intended for debugging and operator inspection — it is not part of the default response to reduce payload size in production traffic.

| Field | Type | Notes |
|---|---|---|
| `strategy` | string | The routing strategy used (e.g. `"best_price"`) |
| `selected_source` | string | The venue ultimately selected for the quote |
| `compared_venues` | array | List of venues considered, with their prices and why they were included or excluded |

---

### 2.4 Exclusion Diagnostics

When one or more venues were evaluated and excluded, the `exclusion_diagnostics` field contains a list of exclusion records. Each record names the venue and carries an `ExclusionReason` variant. The five possible variants are:

| ExclusionReason variant | Plain-English meaning |
|---|---|
| `policy_threshold` | Venue price was outside the configured policy threshold — the spread or price deviation exceeded operator-defined limits |
| `override` | Venue was manually overridden by operator configuration — it has been explicitly excluded via a config rule |
| `stale_data` | Venue's market data exceeded the freshness deadline — the data age was beyond the maximum allowed staleness |
| `circuit_breaker_open` | Venue's circuit breaker is currently open due to recent errors — the venue experienced failures recently and has been temporarily suspended |
| `liquidity_anomaly` | Venue exhibited abnormal liquidity depth — the available depth was either too thin to fill the order or showed a suspicious spike inconsistent with normal market conditions |

If all venues are excluded, the quote will be `degraded: true` or the endpoint will return an error depending on policy configuration.

---

### 2.5 Timestamp Fields

Quote responses carry multiple timestamp fields that operators frequently confuse. This table distinguishes them:

| Field | Type | Meaning |
|---|---|---|
| `timestamp` | int (ms) | Unix milliseconds when this quote response was generated by the API |
| `source_timestamp` | int (ms) | Unix milliseconds of the underlying market data used to compute the quote — reflects how old the orderbook/pool snapshot is |
| `expires_at` | int (ms) | Unix milliseconds after which the client should treat the quote as stale; also the deadline for `POST /api/v1/swap/prepare` |
| `ttl_seconds` | int | Convenience field: number of seconds between `timestamp` and `expires_at` |

---

## 3. AMM Routes: Current Limitation

> **Warning:** `path[].source` values prefixed with `"amm:"` identify AMM (Soroban liquidity pool) routes. Sending an AMM-sourced route to `POST /api/v1/swap/prepare` returns **HTTP 422** with error code `unsupported_execution_mode`. Only routes where every hop has `source == "sdex"` are eligible for prepare today.

**How to identify an AMM hop**

A hop is AMM-sourced when its `source` field starts with the prefix `"amm:"`, for example `"amm:CBLAH...XYZ"`. The suffix is the Soroban liquidity pool contract address. A route is eligible for prepare only if every element of `path[]` has `source == "sdex"` — a single AMM hop anywhere in the route makes the whole route ineligible.

**What `execution_mode: "classic_path_payment"` confirms**

When `POST /api/v1/swap/prepare` succeeds, the `SwapPrepareResponse` always includes `"execution_mode": "classic_path_payment"`. This confirms that the prepared transaction is a `PathPaymentStrictSend` Stellar operation — the classic path-payment type that Freighter, xBull, Albedo, and LOBSTR wallets can sign today.

**AMM Soroban settlement is a gated future milestone.** There is no ETA for AMM route execution from the swap UI. The quote engine surfaces AMM routes because they represent real liquidity that will become executable once the Soroban settlement path is implemented and ungated.

---

## 4. Prepare → Sign → Submit Flow

1. **Prepare** — `POST /api/v1/swap/prepare` with the quote JSON body
   - Do NOT mutate any field of the route JSON before passing to prepare; doing so may produce a rejected or incorrect transaction
   - Verify `network_passphrase` in the response matches your wallet's configured network before proceeding

2. **Sign** — Have the wallet sign `xdr_envelope` from the prepare response
   - Submit the signed envelope, not the unsigned one; submitting the unsigned envelope returns HTTP 400
   - Check `expires_at` in the prepare response; if the current time is past this value, obtain a fresh quote and prepare again (submitting an expired prepare returns HTTP 422 with error code `quote_expired`)

3. **Submit** — `POST /api/v1/swap/submit` with `quote_id` and `signed_xdr`
   - A `quote_id` that has already been submitted returns HTTP 409 with error code `already_submitted`; a fresh prepare is required to retry
   - The `signed_xdr` must be the envelope produced by prepare, signed with the sender wallet

---

## 5. Reading Raw Quote JSON

### 5.1 ApiResponse Envelope

All API responses are wrapped in an `ApiResponse` envelope. The quote object itself lives under the `data` key.

| Field | Notes |
|---|---|
| `v` | Schema version integer — currently `1`; increment signals a breaking envelope change |
| `timestamp` | Unix milliseconds when the response was generated by the server |
| `request_id` | Correlation ID — echoes the `X-Request-ID` header if provided, otherwise server-generated; use this when filing bug reports or tracing logs |
| `data` | The `QuoteResponse` object — all quote fields documented in section 2 live here |

---

### 5.2 Annotated Single-hop SDEX Example

The following is a complete `QuoteResponse` for a testnet XLM → USDC quote. Inline comments (`//`) explain each field.

```jsonc
{
  "v": 1,                                    // schema version
  "timestamp": 1753660800000,                // ms when response was generated
  "request_id": "req_01j9xk6bzv4n9p8m8j1f", // correlation ID
  "data": {
    "base_asset": { "asset_type": "native", "asset_code": null, "asset_issuer": null },
    "quote_asset": {
      "asset_type": "credit_alphanum4",
      "asset_code": "USDC",
      "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
    },
    "amount": "100.0000000",                 // You Pay
    "price": "0.1055000",                    // exchange rate
    "total": "10.5500000",                   // You Receive
    "quote_type": "sell",
    "degraded": false,
    "path": [
      {
        "from_asset": { "asset_type": "native", "asset_code": null, "asset_issuer": null },
        "to_asset": {
          "asset_type": "credit_alphanum4",
          "asset_code": "USDC",
          "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
        },
        "price": "0.1055000",
        "source": "sdex",                    // SDEX hop — eligible for prepare
        "liquidity_depth": "5000.0000000",
        "fee_bps": 30
      }
    ],
    "timestamp": 1753660800000,
    "expires_at": 1753660805000,             // 5 s TTL
    "ttl_seconds": 5,
    "data_freshness": {
      "fresh_count": 3,
      "stale_count": 0,
      "max_staleness_secs": 12
    },
    "price_impact": "0.12",
    "midpoint": "0.1054000",
    "spread_bps": 10
  }
}
```

**Legend**

| JSON field | UI label | Section |
|---|---|---|
| `data.amount` | "You Pay" amount | 2.1 |
| `data.total` | "You Receive" amount | 2.1 |
| `data.price` | Exchange rate | 2.1 |
| `data.quote_type` | Quote type badge | 2.1 |
| `data.base_asset` | From asset | 2.1 |
| `data.quote_asset` | To asset | 2.1 |
| `data.path[]` | Route path | 2.1 / 2.2 |
| `data.expires_at` | Quote expires | 2.1 / 2.5 |
| `data.data_freshness` | Data freshness | 2.1 |
| `data.degraded` | Degraded badge | 2.1 |
| `data.path[].source` | Venue | 2.1 / 3 |
| `data.price_impact` | Price impact | 2.1 |
| `data.midpoint`, `data.spread_bps` | Midpoint / spread | 2.1 |

---

### 5.3 Diagnosing a Degraded Quote

When `degraded` is `true` in a pasted JSON blob, the quote was computed with market data that was soft-stale — the API did not hard-fail the request, but the underlying venue data was older than ideal.

To understand the severity:

- Check `data_freshness.max_staleness_secs` — this is the age of the oldest data point used. A value under 60 seconds is usually acceptable for fast-moving pairs; higher values warrant caution.
- Check `data_freshness.fresh_count` — this shows how many fresh venues were available. A `fresh_count` of zero means all venues were stale and the returned price may differ materially from the true market.
- Check `data_freshness.stale_count` to see how many stale venues contributed to the quote.

Operators MUST NOT resubmit mutated route JSON. If the degraded quote needs to be replaced, obtain a fresh quote from `/api/v1/quote` and run a new prepare. Modifying any field of the route JSON before passing it to prepare is not supported and may produce a rejected or incorrect transaction.

---

## 6. Related Resources

- [Live Swagger UI](/api-docs) — interactive OpenAPI explorer
- [OpenAPI schema source](../../docs/api/openapi.yaml)
- [Error code taxonomy](../api/error_taxonomy.md)
- [Integrator guide](../api/integrator-guide.md)
- [Swap sender-lock recovery runbook](swap-submitting-sender-lock.md)
