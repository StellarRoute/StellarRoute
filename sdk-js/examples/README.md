# sdk-js examples

Copy-paste snippets for the StellarRoute JS/TS SDK. Each file is self-contained and requires no wallet libraries.

## Running an example

```bash
# Install deps from the repo root
npm --prefix sdk-js install

# Run any example with ts-node (ESM)
npx ts-node --esm sdk-js/examples/<file>.ts
```

Set `STELLARROUTE_API_URL` or edit the `baseUrl` constant inside each file to point at a different API host.

---

## Examples

### embed-quote.ts — Fetch a price quote in ~20 lines

The minimal copy-paste snippet. Calls `GET /api/v1/quote/:base/:quote` against the public testnet and prints the result.

```typescript
import { StellarRouteClient } from '@stellarroute/sdk-js';

// Staging: https://52.206.173.91.sslip.io
// Production: https://api.stellarroute.io
const client = new StellarRouteClient('https://52.206.173.91.sslip.io');

const USDC = 'USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN';
const quote = await client.getQuote('native', USDC, 100);

console.log(`100 XLM = ${quote.total} USDC  |  price ${quote.price}`);
```

**Constraints:** read-only — no wallet, no signing, no prepare/submit.

---

### quickstart-health.ts — Health check

Probes `GET /health` and prints per-component status.

### quickstart-pairs.ts — List trading pairs

Fetches active pairs from `GET /api/v1/pairs`.

### quickstart-quote.ts — Price quote (localhost)

Same as `embed-quote.ts` but defaults to `http://localhost:8080` for local development.

### quickstart-routes.ts — Ranked routes

Calls `GET /api/v1/routes/:base/:quote` and prints the top-ranked candidates.

### quickstart-price-history.ts — 24h price history

Calls `GET /api/v1/price-history/:base/:quote` and prints the price series.

### quickstart-simulate.ts — Route dry-run simulation

Calls `POST /api/v1/simulate/route` to validate a route without executing it.

### swap-submit.ts — Full swap flow (prepare → sign → submit)

End-to-end example that builds an unsigned XDR envelope, signs it, and submits. Requires a funded Stellar account and private key — **not for production use without proper key management**.
