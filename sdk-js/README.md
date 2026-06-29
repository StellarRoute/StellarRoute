# StellarRoute TypeScript SDK

Type-safe client for the StellarRoute REST API.

## Install

```bash
npm install @stellarroute/sdk-js
```

## Quickstart

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
