# Telemetry Schema

This document details the telemetry event schema for StellarRoute frontend.
Telemetry is used to understand user interactions and route selection behavior without collecting any sensitive or personally identifiable information (PII).

## Configuration

Telemetry can be disabled by setting the environment variable `NEXT_PUBLIC_TELEMETRY_ENABLED=false`.

## Event Schema (Version 1.0.0)

Every telemetry event follows this base structure:

```typescript
{
  version: "1.0.0",
  eventName: string,
  timestamp: number, // Unix timestamp in milliseconds
  payload: object    // Event-specific data payload
}
```

## Route Selection Events

These events help us understand which routes users are presented with, which ones they select, and which ones they ultimately confirm.

### Allowed Event Names

- `route_view`: Emitted when a user views a route in the UI.
- `route_select`: Emitted when a user selects a specific route from the available options.
- `route_confirm`: Emitted when a user confirms a swap using the selected route.

### Payload Schema

```typescript
{
  fromAssetCode: string; // The asset code being sold (e.g., "XLM", "USDC")
  toAssetCode: string;   // The asset code being bought
  routeLength: number;   // Number of hops in the route (1 = direct trade)
  priceImpactTier: "low" | "medium" | "high" | "severe"; // Categorized price impact
  hasDex: boolean;       // True if the route uses an SDEX orderbook
  hasAmm: boolean;       // True if the route uses a Soroban AMM pool
}
```

### Price Impact Tiers

- `low`: Impact < 0.5%
- `medium`: 0.5% <= Impact < 2.0%
- `high`: 2.0% <= Impact < 5.0%
- `severe`: Impact >= 5.0%

### Sensitive Data Stripping

The payload intentionally excludes:
- Exact trade amounts
- Wallet addresses or public keys
- Identifiable network IP information
- Raw price impact numbers (categorized into tiers instead)
