/**
 * Quickstart: Batch Quote
 *
 * Demonstrates read-only price discovery for multiple pairs in a single request
 * via getQuotesBatch. No swap prepare or wallet interaction.
 */
import { StellarRouteClient, isStellarRouteApiError } from '../src/index.js';
import type { QuoteRequestItem } from '../src/index.js';

const client = new StellarRouteClient('http://localhost:8080');

// Replace with real USDC/yXLM issuers on the network you are testing against.
const USDC_ISSUER = 'GDUKMGUGDZQK6YH...';
const YXLM_ISSUER = 'GARDNV3Q7YGT4AKSDF25LT32YSCCW4EV22Y2TV3I2PU2MMXJTEDL5T55';
const USDC  = `USDC:${USDC_ISSUER}`;
const YXLM  = `yXLM:${YXLM_ISSUER}`;

const requests: QuoteRequestItem[] = [
  // Sell 100 XLM, receive USDC
  { base: 'native', quote: USDC,  amount: 100, quote_type: 'sell' },
  // Sell 50 USDC, receive XLM
  { base: USDC,     quote: 'native', amount: 50, quote_type: 'sell' },
  // Sell 200 XLM, receive yXLM
  { base: 'native', quote: YXLM,  amount: 200, quote_type: 'sell' },
];

async function main(): Promise<void> {
  console.log(`Fetching ${requests.length} quotes in a single batch request...`);

  let result;
  try {
    result = await client.getQuotesBatch(requests);
  } catch (err) {
    if (isStellarRouteApiError(err)) {
      console.error(`Batch request failed [${err.code}]: ${err.message}`);
      process.exitCode = 1;
      return;
    }
    throw err;
  }

  console.log(`\nBatch quotes (${result.total} returned)`);
  console.log('--------------------------------------');
  for (let i = 0; i < result.quotes.length; i++) {
    const req   = requests[i];
    const quote = result.quotes[i];

    const baseLabel  = req.base  === 'native' ? 'XLM' : req.base.split(':')[0];
    const quoteLabel = req.quote === 'native' ? 'XLM' : req.quote.split(':')[0];

    console.log(
      `[${i}] ${baseLabel}/${quoteLabel}  ` +
      `amount=${quote.amount}  price=${quote.price}  total=${quote.total}  ` +
      `hops=${quote.path.length}` +
      (quote.price_impact ? `  impact=${quote.price_impact}%` : ''),
    );
  }
}

main().catch((error) => {
  console.error('Quickstart batch-quote example failed:', error);
  process.exitCode = 1;
});
