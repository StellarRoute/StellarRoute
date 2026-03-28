# `/api/v1/routes` — Multi-Route Trading Endpoint

Returns multiple **ranked route candidates** for a given trading pair. Unlike the single `/quote` endpoint, this endpoint exposes the full set of execution paths scored by the `HybridOptimizer`, enabling clients (UI, SDKs) to display alternative routes and let users choose.

---

## Endpoint

```
GET /api/v1/routes/:base/:quote
```

| Segment | Description |
|---------|-------------|
| `:base` | Selling asset — `native`, `USDC`, or `USDC:ISSUER` |
| `:quote` | Buying asset — same format as `:base` |

---

## Query Parameters

| Parameter | Type | Default | Max | Description |
|-----------|------|---------|-----|-------------|
| `amount` | `f64` | `1` | — | Trade amount (in asset units) |
| `limit` | `usize` | `5` | `20` | Maximum number of routes to return |
| `max_hops` | `usize` | `3` | `6` | Maximum hops per route |
| `environment` | `string` | `production` | — | Optimizer policy (`production`, `testnet`) |

---

## Example Request

```
GET /api/v1/routes/native/USDC:GA5ZSEJ...?amount=100&limit=3&max_hops=2
```

---

## Response Shape

```jsonc
{
  "base_asset":  { "asset_type": "native" },
  "quote_asset": { "asset_type": "credit_alphanum4", "asset_code": "USDC", "asset_issuer": "GA5Z..." },
  "amount":      "100.0000000",
  "timestamp":   1711526277000,
  "routes": [
    {
      "score":            94.5,       // Higher is better
      "impact_bps":       30,         // Price impact in basis points
      "estimated_output": "99.7000000",
      "policy_used":      "production",
      "path": [
        {
          "from_asset":        { "asset_type": "native" },
          "to_asset":          { "asset_type": "credit_alphanum4", "asset_code": "USDC" },
          "price":             "1.0000000",
          "amount_out_of_hop": "99.7000000",
          "fee_bps":           30,
          "source":            "amm:pool-abc123"
        }
      ]
    },
    {
      "score":            87.2,
      "impact_bps":       40,
      "estimated_output": "99.6000000",
      "policy_used":      "production",
      "path": [
        { "from_asset": ..., "to_asset": ..., "fee_bps": 20, "source": "sdex" },
        { "from_asset": ..., "to_asset": ..., "fee_bps": 20, "source": "sdex" }
      ]
    }
  ]
}
```

---

## Ranking Logic

Routes are ranked by their **composite score**, computed by the `HybridOptimizer` using:

- **Estimated output** — higher output = better score
- **Price impact** (`impact_bps`) — lower impact = better score
- **Hop count** — fewer hops preferred unless output significantly improves
- **Policy weights** — configurable per environment (`production` vs `testnet`)

The first route in the array is always the recommended best route.

---

## Per-Hop Metadata

Each `path` entry exposes:

| Field | Description |
|-------|-------------|
| `from_asset` | Input asset for this swap leg |
| `to_asset` | Output asset for this swap leg |
| `price` | Exchange rate at this hop (7 decimal places) |
| `amount_out_of_hop` | Expected output amount after fees |
| `fee_bps` | Fee charged by this venue in basis points |
| `source` | Venue identifier (`sdex`, `amm:pool-id`) |

---

## Error Responses

| HTTP Code | Condition |
|-----------|-----------|
| `400` | Invalid asset format, non-positive amount, or unknown environment |
| `404` | No executable route found between the two assets |
| `500` | Internal graph or computation failure |

---

## Implementation Notes

- **Zero DB per request**: The routing graph is maintained in-memory by the background `GraphManager`, syncing from the database every 5 seconds.
- **Thread safety**: BFS pathfinding runs inside `tokio::spawn_blocking` to prevent blocking the async runtime.
- **Deduplication**: Concurrent identical requests are collapsed via `SingleFlight` to prevent thundering herd.
- **OpenAPI**: Documented via `utoipa::path` — visible in the `/docs` Swagger UI.
