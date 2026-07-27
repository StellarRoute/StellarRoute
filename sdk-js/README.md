# StellarRoute TypeScript SDK

Type-safe client for the StellarRoute REST API.

## Install

```bash
npm install @stellarroute/sdk-js
```

## Quickstart: quote → swap

The full integration path is three calls — price the trade, pick a route, execute it.

```ts
import { StellarRouteClient, isStellarRouteApiError } from '@stellarroute/sdk-js';

const client = new StellarRouteClient('https://api.stellarroute.io');

// 1. Quote — what will this trade cost?
const quote = await client.getQuote('native', 'USDC:GDUKMGUGDZQK6YH...', 100, 'sell');
console.log(`price=${quote.price} total=${quote.total}`);

// 2. Routes — rank the executable paths for that pair and amount.
const { routes } = await client.getRankedRoutes('native', 'USDC:GDUKMGUGDZQK6YH...', 100, 5);
const best = routes[0];

// 3. Swap — simulate + build the transaction envelope.
try {
  const result = await client.executeSwap({
    route: { hops: best.path.map((hop) => ({
      from_asset: hop.from_asset,
      to_asset: hop.to_asset,
      source: hop.source,
    })) },
    amount: '100',
    sender: 'GABC...',
    slippage_bps: 50,
  });

  // Sign `result.xdr_envelope` with the Stellar SDK and submit it.
} catch (err) {
  if (isStellarRouteApiError(err) && err.code === 'not_implemented') {
    // Simulation passed; the swap-build endpoint is not deployed yet.
    // Build and sign the transaction directly via the Stellar SDK.
  }
}
```

Releases follow SemVer — see [CHANGELOG.md](./CHANGELOG.md) for breaking changes and
[PUBLISHING.md](./PUBLISHING.md) for the release checklist.

## Reference

### Get a quote

```ts
import { StellarRouteClient } from '@stellarroute/sdk-js';

const client = new StellarRouteClient('http://localhost:8080');
const quote = await client.getQuote(
  'native',
  'USDC:GDUKMGUGDZQK6YH...',
  100,
  'sell',
);

console.log(quote.price, quote.total);
```

### Get ranked routes

```ts
import { StellarRouteClient } from '@stellarroute/sdk-js';

const client = new StellarRouteClient('http://localhost:8080');
const result = await client.getRankedRoutes(
  'native',
  'USDC:GDUKMGUGDZQK6YH...',
  100,
  5, // limit
);

result.routes.forEach((route) => {
  console.log(`score=${route.score} output=${route.estimated_output}`);
  route.path.forEach((hop) => console.log(`  ${hop.source}: ${hop.price}`));
});
```

### Simulate a route (dry-run)

```ts
import { StellarRouteClient } from '@stellarroute/sdk-js';

const client = new StellarRouteClient('http://localhost:8080');
const result = await client.simulateRoute({
  route: {
    hops: [
      { from_asset: 'native', to_asset: 'USDC:GDUKMGUGDZQK6YH...', source: 'sdex' },
    ],
  },
  amount: '100',
  slippage_bps: 50,
});

console.log(result.quote.total);
if (result.exclusion_diagnostics) {
  result.exclusion_diagnostics.excluded_venues.forEach((v) => {
    console.log(`excluded ${v.venue_ref}: ${v.reason}`);
  });
}
```

Additional runnable quickstart files are in `sdk-js/examples/`.

## API docs

Generate TypeDoc API docs:

```bash
npm run docs:api
```

Generated docs are published in `docs/sdk-js/api/`.

## Error handling

For integration guidance on retry semantics, SDK error helper usage, and user-facing error patterns, see `docs/api/integrator-error-guide.md`.
