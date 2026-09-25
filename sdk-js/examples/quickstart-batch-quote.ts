import { StellarRouteClient } from '../src/index.js';

const client = new StellarRouteClient('http://localhost:8080');

async function main(): Promise<void> {
  const response = await client.getQuotesBatch([
    {
      base: 'native',
      quote: 'USDC:GDUKMGUGDZQK6YH...',
      amount: 100,
      quote_type: 'sell',
    },
    {
      base: 'native',
      quote: 'USDC:GDUKMGUGDZQK6YH...',
      amount: 250,
      quote_type: 'buy',
    },
  ]);

  console.log('Batch quote results');
  console.log('-------------------');
  response.quotes.forEach((quote, index) => {
    console.log(`Request ${index + 1}: ${quote.amount} ${quote.base_asset.asset_type === 'native' ? 'XLM' : quote.base_asset.asset_code ?? 'UNKNOWN'} -> ${quote.total} ${quote.quote_asset.asset_type === 'native' ? 'XLM' : quote.quote_asset.asset_code ?? 'UNKNOWN'} @ ${quote.price}`);
  });
}

main().catch((error) => {
  console.error('Quickstart batch quote example failed:', error);
  process.exitCode = 1;
});
