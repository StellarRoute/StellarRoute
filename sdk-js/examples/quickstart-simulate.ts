/**
 * quickstart-simulate.ts
 *
 * Demonstrates simulateRoute (dry-run a route before executing) and
 * executeSwap (build a signed-ready XDR envelope).
 *
 * Run after starting the local API:
 *   cargo run -p stellarroute-api
 *   npx ts-node --esm sdk-js/examples/quickstart-simulate.ts
 */

import { StellarRouteClient, isStellarRouteApiError } from '../src/index.js';
import type { SimulateRouteRequest } from '../src/index.js';

const client = new StellarRouteClient('http://localhost:8080');

async function main(): Promise<void> {
  // 1. Build a route to simulate (single-hop XLM → USDC via SDEX).
  const request: SimulateRouteRequest = {
    route: {
      hops: [
        {
          from_asset: 'native',
          to_asset: 'USDC:GDUKMGUGDZQK6YHKQ9L7BWHY...',
          source: 'sdex',
        },
      ],
    },
    amount: '100',
    slippage_bps: 50, // 0.5 %
  };

  // 2. Dry-run via simulateRoute.
  console.log('Simulating route (dry-run)...');
  const simulation = await client.simulateRoute(request);
  const { quote } = simulation;

  console.log('\nSimulation result');
  console.log('-----------------');
  console.log(`Input:         ${quote.amount} XLM`);
  console.log(`Expected out:  ${quote.total} USDC`);
  console.log(`Effective price: ${quote.price}`);
  if (quote.price_impact) {
    console.log(`Price impact:  ${quote.price_impact}%`);
  }
  if (simulation.exclusion_diagnostics?.excluded_venues.length) {
    console.log(
      `Excluded venues: ${simulation.exclusion_diagnostics.excluded_venues
        .map((v) => `${v.venue_ref} (${v.reason})`)
        .join(', ')}`,
    );
  }

  // 3. Attempt executeSwap — currently returns a documented stub error until
  //    the swap-build endpoint is deployed.
  console.log('\nAttempting executeSwap...');
  try {
    const swapResult = await client.executeSwap({
      route: request.route,
      amount: request.amount,
      sender: 'GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX',
      min_output: String(Number(quote.total) * 0.995), // 0.5 % slippage guard
      slippage_bps: 50,
    });
    // When the endpoint ships, sign and submit swapResult.xdr_envelope.
    console.log('XDR envelope ready to sign:', swapResult.xdr_envelope.slice(0, 40) + '…');
  } catch (err) {
    if (isStellarRouteApiError(err) && err.code === 'not_implemented') {
      console.log(
        'executeSwap is not yet available on this API instance.\n' +
          'Simulation succeeded — build and sign the transaction via the Stellar SDK.',
      );
    } else {
      throw err;
    }
  }
}

main().catch((error) => {
  console.error('quickstart-simulate failed:', error);
  process.exitCode = 1;
});
