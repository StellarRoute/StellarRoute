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

// 3. Swap — prepare → network check → sign once → submit.
try {
  const result = await client.executeSwap({
    route: { hops: best.path.map((hop) => ({
      from_asset: hop.from_asset.asset_type === 'native'
        ? 'native'
        : `${hop.from_asset.asset_code}:${hop.from_asset.asset_issuer}`,
      to_asset: hop.to_asset.asset_type === 'native'
        ? 'native'
        : `${hop.to_asset.asset_code}:${hop.to_asset.asset_issuer}`,
      source: hop.source,
    })) },
    amount: '100',
    sender: 'GABC...',
    slippage_bps: 50,
    // Required: current wallet/app passphrase (or async getter).
    // Mismatch → typed `network_mismatch` before sign/submit.
    networkPassphrase: 'Test SDF Network ; September 2015',
    signTransaction: async (xdr) => {
      // Freighter / wallet sign; pass the same networkPassphrase to the wallet.
      return xdr;
    },
  });

  console.log(`tx_hash=${result.tx_hash}`);
} catch (err) {
  if (isStellarRouteApiError(err) && err.code === 'network_mismatch') {
    // Wallet is on the wrong network — switch and refresh the quote.
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

Additional runnable quickstart files are in `sdk-js/examples/`:

- [Health check](./examples/quickstart-health.ts)
- [Trading pairs](./examples/quickstart-pairs.ts)
- [Orderbook snapshot](./examples/quickstart-orderbook.ts)
- [Batch quote](./examples/quickstart-batch-quote.ts)

See the [price history example](./examples/quickstart-price-history.ts) for a read-only 24-hour history query.

## API docs

Generate TypeDoc API docs:

```bash
npm run docs:api
```

Generated docs are published in `docs/sdk-js/api/`.

## Error handling

For integration guidance on retry semantics, SDK error helper usage, and user-facing error patterns, see `docs/api/integrator-error-guide.md`.
