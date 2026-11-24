/**
 * Quickstart: Orderbook
 *
 * Demonstrates read-only orderbook snapshots: a single pair via
 * `getOrderbook`, and multiple pairs in one request via
 * `getOrderbooksBatch`. No swap prepare or wallet interaction.
 */
import { StellarRouteClient, isStellarRouteApiError } from '../src/index.js';
import type { OrderbookRequestItem } from '../src/index.js';

export const client = new StellarRouteClient('http://localhost:8080');

// Replace with real asset issuers on the network you are testing against.
const USDC_ISSUER = 'GDUKMGUGDZQK6YH...';
const YXLM_ISSUER = 'GARDNV3Q7YGT4AKSDF25LT32YSCCW4EV22Y2TV3I2PU2MMXJTEDL5T55';
export const USDC = `USDC:${USDC_ISSUER}`;
export const YXLM = `yXLM:${YXLM_ISSUER}`;

function describeLevels(label: string, levels: { price: string; amount: string }[]): void {
  console.log(`  ${label}:`);
  for (const level of levels.slice(0, 3)) {
    console.log(`    price=${level.price} amount=${level.amount}`);
  }
  if (levels.length === 0) {
    console.log('    (none)');
  }
}

export async function singleOrderbook(): Promise<void> {
  console.log('1. Single orderbook (getOrderbook)');
  console.log('-----------------------------------');

  try {
    const orderbook = await client.getOrderbook('native', USDC);
    describeLevels('bids', orderbook.bids);
    describeLevels('asks', orderbook.asks);
  } catch (err) {
    if (isStellarRouteApiError(err) && err.isNotFound()) {
      console.log('  no active offers for this pair');
      return;
    }
    throw err;
  }
}

export async function batchOrderbook(): Promise<void> {
  console.log('\n2. Batch orderbook (getOrderbooksBatch)');
  console.log('----------------------------------------');

  const requests: OrderbookRequestItem[] = [
    { base: 'native', quote: USDC },
    { base: 'native', quote: YXLM },
  ];

  let result;
  try {
    result = await client.getOrderbooksBatch(requests);
  } catch (err) {
    if (isStellarRouteApiError(err)) {
      console.error(`Batch request failed [${err.code}]: ${err.message}`);
      process.exitCode = 1;
      return;
    }
    throw err;
  }

  console.log(
    `Batch orderbooks: ${result.items_succeeded} ok, ${result.items_failed} failed, ${result.total} total`,
  );
  for (const item of result.results) {
    const req = requests[item.index];
    const quoteLabel = req.quote === 'native' ? 'XLM' : req.quote.split(':')[0];

    if (item.status === 'ok' && item.orderbook) {
      console.log(
        `[${item.index}] native/${quoteLabel}: ${item.orderbook.bids.length} bid levels, ` +
          `${item.orderbook.asks.length} ask levels`,
      );
    } else {
      console.log(`[${item.index}] native/${quoteLabel}: error ${item.error?.code ?? 'unknown'}`);
    }
  }
}

export async function main(): Promise<void> {
  await singleOrderbook();
  await batchOrderbook();
}

// Only auto-run when executed directly (e.g. `tsx quickstart-orderbook.ts`),
// not when imported by a test.
const isMain = process.argv[1] && import.meta.url === `file://${process.argv[1]}`;
if (isMain) {
  main().catch((error) => {
    console.error('Quickstart orderbook example failed:', error);
    process.exitCode = 1;
  });
}
