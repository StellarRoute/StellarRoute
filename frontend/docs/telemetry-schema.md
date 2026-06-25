# Telemetry Schema

This document details the telemetry events fired by the StellarRoute frontend.

---

## Route Selection Event

* **Event Name**: `stellarroute:route-selected`
* **Trigger**: Fired when a user selects an alternative route in the routing/swap UI.
* **Environment Guard**: Respects `NEXT_PUBLIC_TELEMETRY_ENABLED`. If set to `false`, no telemetry events are dispatched.

### Payload Fields

| Field | Type | Description |
|---|---|---|
| `venue` | `string` | The liquidity venue or pool name of the selected route (e.g. `AQUA Pool`, `SDEX`, `Blend Pool`, `Phoenix AMM`). |
| `hopCount` | `number` | The number of hops in the selected routing path (e.g. `1` for direct swaps, `2` or more for multi-hop swaps). |

---

## Quote Retry Event

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
