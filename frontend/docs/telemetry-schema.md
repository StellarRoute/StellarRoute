# Telemetry Schema

This document details the telemetry event schemas for the StellarRoute frontend.
Telemetry is used to understand user interactions and route selection behavior without collecting any sensitive or personally identifiable information (PII).

---

## 1. Route Selection Event

* **Event Name**: `stellarroute:route-selected`
* **Trigger**: Fired when a user selects a specific route (or alternative route) from the available options in the routing/swap UI.
* **Environment Guard**: Respects `NEXT_PUBLIC_TELEMETRY_ENABLED`. If set to `false`, no telemetry events are dispatched.

### Payload Fields

| Field | Type | Description |
|---|---|---|
| `venue` | `string` | The liquidity venue or pool name of the selected route (e.g. `AQUA Pool`, `SDEX`, `Blend Pool`, `Phoenix AMM`). |
| `hopCount` | `number` | The number of hops in the selected routing path (e.g. `1` for direct swaps, `2` or more for multi-hop swaps). |

---

## 2. Quote Retry Event

* **Event Name**: `stellarroute:quote-retry`
* **Trigger**: Fired during quote refresh retry cycles (scheduled, cancelled, succeeded, or failed).

### Payload Fields

| Field | Type | Description |
|---|---|---|
| `stage` | `'scheduled' \| 'cancelled' \| 'succeeded' \| 'failed'` | The stage of the retry event. |
| `request` | `QuoteRetryRequestContext` | The request context (assets, amount, quote type). |
| `attempt` | `number` | The retry attempt count. |
| `delayMs` | `number` | The delay in milliseconds before the retry. |
| `errorMessage` | `string` | (Optional) Error message on failure. |

---

## 3. Swap Funnel Events

* **Event Name**: `stellarroute:swap-funnel`
* **Trigger**: Fired at key live-swap debugging checkpoints (quote → confirm → submit → settle).
* **Environment Guard**: Respects `NEXT_PUBLIC_TELEMETRY_ENABLED`. If set to `false`, no telemetry events are dispatched.

### `eventName` values

| eventName | When |
|---|---|
| `quote_requested` | A quote HTTP request is about to be sent |
| `confirm_clicked` | User confirms the swap (post high-impact gate if any) |
| `swap_submitted` | Signed envelope handed to Horizon broadcast |
| `swap_finalized` | Horizon submit returns a transaction hash (confirmed) |
| `swap_failed` | Build, sign, or submit path fails |

### Envelope

| Field | Type | Description |
|---|---|---|
| `version` | `'1.0.0'` | Schema version |
| `eventName` | `SwapFunnelEventName` | One of the funnel stages above |
| `timestamp` | `number` | Epoch ms when the event was emitted |
| `payload` | `SwapFunnelPayload` | PII-safe context (see below) |

### Payload Fields

| Field | Type | Description |
|---|---|---|
| `quoteId` | `string` | Opaque quote/request id when known |
| `routeId` | `string` | Selected route id when known |
| `fromAssetCode` | `string` | Source asset code / native identifier |
| `toAssetCode` | `string` | Destination asset code / native identifier |
| `hopCount` | `number` | Path length |
| `priceImpactTier` | `'low' \| 'medium' \| 'high' \| 'severe'` | Categorized impact (never raw %) |
| `failureStage` | `string` | Coarse stage only (`build`, `sign`, `submit`, `config`) on `swap_failed` |

---

## Sensitive Data Stripping

The payload intentionally excludes:
- Exact trade amounts
- Wallet addresses or public keys
- Identifiable network IP information
- Raw price impact numbers (categorized into tiers instead)
- Full error strings that may embed account ids or XDR
