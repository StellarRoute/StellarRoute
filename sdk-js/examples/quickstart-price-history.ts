import { StellarRouteClient } from '../src/index.js';

const API_URL = process.env.STELLARROUTE_API_URL ?? 'http://localhost:8080';
const BASE_ASSET = 'native';
const QUOTE_ASSET = 'USDC:GDUKMGUGDZQK6YH...';

async function main(): Promise<void> {
  const client = new StellarRouteClient({ baseUrl: API_URL });
  const history = await client.getPriceHistory(BASE_ASSET, QUOTE_ASSET, {
    window: '24h',
  });

  console.log(`${history.base_asset.asset_code ?? 'XLM'} / ${history.quote_asset.asset_code ?? QUOTE_ASSET}`);
  console.log(`Window: ${history.window}`);
  console.log(`Source: ${history.source}`);
  console.log(`Generated: ${new Date(history.generated_at).toISOString()}`);
  console.log('Price history');
  console.log('-------------');

  history.points.forEach((point) => {
    console.log(`${new Date(point.timestamp).toISOString()}: ${point.price}`);
  });
}

main().catch((error) => {
  console.error('Quickstart price history example failed:', error);
  process.exitCode = 1;
});
