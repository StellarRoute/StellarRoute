import { StellarRouteClient } from '../src/index.js';

const client = new StellarRouteClient('http://localhost:8080');

async function main(): Promise<void> {
  const result = await client.getPairs();

  console.log(`Active trading pairs: ${result.total}`);
  console.log('----------------------');
  for (const pair of result.pairs) {
    console.log(
      `${pair.base} / ${pair.counter}: ${pair.offer_count} offers` +
        (pair.last_updated ? ` (updated ${pair.last_updated})` : ''),
    );
  }
}

main().catch((error) => {
  console.error('Quickstart pairs example failed:', error);
  process.exitCode = 1;
});
