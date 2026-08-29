import { StellarRouteClient } from '../src/index.js';

const client = new StellarRouteClient('http://localhost:8080');

function assetSymbol(asset: { asset_type: string; asset_code?: string | null; asset_issuer?: string | null }): string {
  if (asset.asset_type === 'native') {
    return 'XLM';
  }

  return asset.asset_code ?? 'UNKNOWN';
}

async function main(): Promise<void> {
  const orderbook = await client.getOrderbook('native', 'USDC:GDUKMGUGDZQK6YH...');

  console.log('Orderbook snapshot');
  console.log('------------------');
  console.log(`Base asset: ${assetSymbol(orderbook.base_asset)}`);
  console.log(`Quote asset: ${assetSymbol(orderbook.quote_asset)}`);
  console.log(`Bid levels: ${orderbook.bids.length}`);
  console.log(`Ask levels: ${orderbook.asks.length}`);

  if (orderbook.bids.length > 0) {
    const bestBid = orderbook.bids[0];
    console.log(`Best bid: ${bestBid.price} ${assetSymbol(orderbook.quote_asset)} / ${assetSymbol(orderbook.base_asset)}`);
  }

  if (orderbook.asks.length > 0) {
    const bestAsk = orderbook.asks[0];
    console.log(`Best ask: ${bestAsk.price} ${assetSymbol(orderbook.quote_asset)} / ${assetSymbol(orderbook.base_asset)}`);
  }
}

main().catch((error) => {
  console.error('Quickstart orderbook example failed:', error);
  process.exitCode = 1;
});
