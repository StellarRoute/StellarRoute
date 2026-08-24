# Price History Sparkline Feature

## Overview

The price history sparkline shows a lightweight **24-hour mid-price trend** for the active trading pair. It is designed for compact UI surfaces such as the swap panel and orderbook page, where traders need quick context without loading a full charting library.

The feature is split into three layers:

| Layer | File | Responsibility |
|-------|------|----------------|
| API hook | `frontend/hooks/useApi.ts` → `usePriceHistory` | Fetch and auto-refresh series data |
| API client | `frontend/lib/api/client.ts` → `getPriceHistory` | HTTP request to the backend |
| Presentation | `frontend/components/shared/PriceHistorySparkline.tsx` | SVG sparkline, hover tooltips, empty/loading states |

Related but separate: `PriceSparkline.tsx` is a multi-range (`1h` / `24h` / `7d`) chart that accepts pre-fetched `rangeData`. The price-history feature documented here is the **API-backed 24h sparkline** built around `usePriceHistory` + `PriceHistorySparkline`.

## Issue Details

- **Issue**: #792 [documentation] Write price-history feature doc for sparkline API
- **Milestone**: M4 — Orderbook UI
- **Complexity**: Hard
- **Status**: Documented

---

## API Contract

### Endpoint

```
GET /api/v1/price-history/{base}/{quote}
```

Canonical reference: [`docs/api/openapi.yaml`](../../docs/api/openapi.yaml) (`get_price_history`) and backend handler `crates/api/src/routes/price_history.rs`.

### Path parameters

| Parameter | Description | Examples |
|-----------|-------------|----------|
| `base` | Base asset identifier (URL-encoded) | `native`, `USDC`, `USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN` |
| `quote` | Quote asset identifier (URL-encoded) | Same format as `base` |

Asset strings follow the same convention used elsewhere in the frontend (`fromToken` / `toToken` in `SwapCard`, `base_asset` / `counter_asset` on trading pairs):

- **Native XLM**: `native`
- **Issued assets**: `CODE` or `CODE:ISSUER` (issuer required when multiple assets share a code)

There are **no query parameters**. The window is fixed at 24 hours.

### Response shape

```typescript
interface PriceHistoryResponse {
  base_asset: Asset;
  quote_asset: Asset;
  window: "24h";
  source: string;           // e.g. "orderbook_snapshots.mid_price"
  generated_at: number;     // Unix ms when the series was built
  points: PriceHistoryPoint[];
}

interface PriceHistoryPoint {
  timestamp: number;        // Unix ms — start of hourly bucket
  price: string;            // Average mid-price for the bucket (decimal string)
}
```

### Backend behaviour

- Aggregates `orderbook_snapshots.mid_price` into **hourly buckets** over the trailing 24 hours (max 24 points).
- Returns HTTP `200` with an **empty `points` array** when the pair exists but no usable snapshots were recorded in the window.
- Responses are cached server-side for **30 seconds** (`price-history:{base}:{quote}` Redis key).
- Error responses:
  - `400` — invalid asset identifier
  - `404` — asset or trading pair not found
  - `500` — internal error

See also: [`docs/architecture/database-schema.md`](../../docs/architecture/database-schema.md#price-history-contract).

### Refresh interval

| Layer | Default interval | Configurable? |
|-------|------------------|---------------|
| Frontend hook (`usePriceHistory`) | **60 seconds** (`60_000` ms) | Yes — pass `refreshIntervalMs` as the third argument |
| Backend Redis cache | 30 seconds | No (server-managed) |
| Orderbook hook (`useOrderbook`) | 10 seconds | Yes — used for comparison when wiring orderbook page |

The hook skips fetching when `base` or `quote` is empty, or when `skip` is `true`:

```typescript
usePriceHistory(base, quote, refreshIntervalMs?, skip?)
```

`usePriceHistory` does **not** show error toasts by default (`showToastOnError: false`). Callers decide how to surface fetch failures in context.

---

## Frontend Data Flow

```
Trading pair (base, quote)
        │
        ▼
 usePriceHistory(base, quote)          ← auto-refresh every 60s
        │
        ▼
 stellarRouteClient.getPriceHistory()  ← GET /api/v1/price-history/{base}/{quote}
        │
        ▼
 PriceHistorySparkline                 ← renders last ≤24 points
   points={data?.points ?? []}
   loading={loading}
```

The sparkline component keeps at most the **last 24 points** client-side (`points.slice(-24)`) even if the API returns more.

---

## Wiring Guide

### SwapCard (`frontend/components/swap/SwapCard.tsx`)

`SwapCard` already tracks `fromToken` and `toToken` and broadcasts the pair through `TradingPairContext`. To add the sparkline below the token inputs:

```tsx
import { PriceHistorySparkline } from "@/components/shared/PriceHistorySparkline";
import { usePriceHistory } from "@/hooks/useApi";

// Inside SwapCard, after fromToken / toToken are available:
const {
  data: priceHistory,
  loading: priceHistoryLoading,
  error: priceHistoryError,
} = usePriceHistory(fromToken, toToken);

// In JSX (e.g. below the amount fields):
<PriceHistorySparkline
  points={priceHistory?.points}
  loading={priceHistoryLoading}
  title="24h price trend"
/>
```

**Pair direction:** pass `fromToken` as `base` and `toToken` as `quote` to match swap semantics (price of base in quote terms). The series reflects the indexed trading pair orientation in the database; if the API returns `404` for the forward pair, try the reverse orientation or hide the sparkline.

**When to skip:** pass `skip: true` (fourth argument) while tokens are unset or during storybook fixtures that mock fetch.

### Orderbook page (`frontend/app/orderbook/page.tsx`)

The orderbook page already loads the selected pair via `selectedPair?.base_asset` and `selectedPair?.counter_asset`, and syncs highlighting with `useOptionalTradingPair()`.

Place the sparkline near `MarketDepthChart` so depth and trend share the same pair:

```tsx
import { PriceHistorySparkline } from "@/components/shared/PriceHistorySparkline";
import { usePriceHistory } from "@/hooks/useApi";

const {
  data: priceHistory,
  loading: priceHistoryLoading,
} = usePriceHistory(
  selectedPair?.base_asset ?? "",
  selectedPair?.counter_asset ?? "",
);

// Above or below MarketDepthChart:
<PriceHistorySparkline
  points={priceHistory?.points}
  loading={priceHistoryLoading}
  className="mb-4"
/>
```

**Context sync:** when the user changes the pair selector, `selectedPair` updates and `usePriceHistory` re-fetches automatically because `base` / `quote` are hook dependencies. When the swap panel highlights the orderbook via `TradingPairContext`, no extra sparkline wiring is required — both views read the same `base` / `quote` strings.

**Refresh alignment:** orderbook rows refresh every 10s (`useOrderbook`); price history refreshes every 60s by default. Keep these intervals separate — the sparkline is a coarse hourly trend, not a live quote.

### TradingPairContext (optional)

For a global sparkline outside `SwapCard` / orderbook, read the pair from context:

```tsx
const tradingPair = useOptionalTradingPair();
const base = tradingPair?.fromAsset ?? "";
const quote = tradingPair?.toAsset ?? "";
const { data, loading } = usePriceHistory(base, quote);
```

---

## Error and Fallback Behaviour

### `usePriceHistory` hook

| State | `data` | `loading` | `error` | Notes |
|-------|--------|-----------|---------|-------|
| Initial fetch | `undefined` | `true` | `null` | Shows loading UI |
| Success | `PriceHistoryResponse` | `false` | `null` | Pass `data.points` to sparkline |
| HTTP / network failure | `undefined` | `false` | `StellarRouteApiError` or `Error` | No toast unless caller adds one |
| Skipped (`skip: true` or empty pair) | `undefined` | `false` | `null` | Hook idle — render nothing or a placeholder |
| Unmount / pair change | — | — | — | In-flight request aborted via `AbortController` |

On error, prefer **hiding the sparkline** or showing a compact inline message rather than blocking the swap/orderbook flow. The hook exposes `refresh()` for manual retry.

### `PriceHistorySparkline` component

| Condition | UI |
|-----------|-----|
| `loading={true}` | Pulsing skeleton (title bar + chart area) |
| `points` empty or all invalid | Dashed border card with `emptyLabel` (default: *"No 24h price data available yet."*) |
| Valid points | SVG area chart with hover/focus tooltips showing time + approximate price |
| Flat price range (min === max) | Line rendered at vertical centre (`y = 50`) |

Props for customisation:

```typescript
interface PriceHistorySparklineProps {
  points?: PriceHistoryPoint[];
  loading?: boolean;
  className?: string;
  title?: string;        // default: "24h price trend"
  emptyLabel?: string;   // default: "No 24h price data available yet."
}
```

**Accessibility:** chart has `role="img"` and `aria-label="24 hour price sparkline"`; each sample is a focusable `button` with an `aria-label` describing timestamp and price.

---

## Testing

### Component tests

`frontend/components/shared/PriceHistorySparkline.test.tsx` covers:

- Empty state copy
- Latest approximate price in header
- Hover tooltip with time and price

Run:

```bash
cd frontend
npm test -- PriceHistorySparkline
```

### Manual verification

1. Start the API with indexed `orderbook_snapshots` data for a pair.
2. `curl http://localhost:8080/api/v1/price-history/native/USDC` — confirm `points` array.
3. Wire the component in swap or orderbook and confirm auto-refresh after 60s.

---

## Related Files

- `frontend/hooks/useApi.ts` — `usePriceHistory` hook
- `frontend/lib/api/client.ts` — `getPriceHistory` client method
- `frontend/components/shared/PriceHistorySparkline.tsx` — sparkline UI
- `frontend/types/index.ts` — `PriceHistoryPoint`, `PriceHistoryResponse`
- `frontend/components/swap/SwapCard.tsx` — primary swap integration point
- `frontend/app/orderbook/page.tsx` — orderbook integration point
- `frontend/contexts/TradingPairContext.tsx` — shared pair state
- `docs/api/openapi.yaml` — OpenAPI spec
- `crates/api/src/routes/price_history.rs` — backend handler

## Documentation Links

- [StellarRoute docs hub — Frontend feature docs](../../docs/README.md#frontend-feature-docs)
- [Orderbook highlighting feature](./orderbook-highlighting-feature.md) — `TradingPairContext` wiring pattern
- [Database schema — price history contract](../../docs/architecture/database-schema.md)
