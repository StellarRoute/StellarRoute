import { StellarRouteClient, isStellarRouteApiError } from '../src/index.js';

/**
 * Classic one-hop prepare → sign → submit → confirm example.
 *
 * Sign with Freighter in the browser (or another wallet). This example does
 * not load secret keys from the environment.
 */
const SENDER_PUBLIC_KEY = 'GABC...'; // Replace with a real testnet public key
const USDC_TESTNET = 'USDC:GBBD47IF6LWK7P7MDEVSCWTTCJMPN2S4RY3G5GNCY7G1MNC2S4RY3G5G';
const API_URL = process.env.STELLARROUTE_API_URL ?? 'http://localhost:8080';
const HORIZON_URL =
  process.env.STELLAR_HORIZON_URL ?? 'https://horizon-testnet.stellar.org';

async function main() {
  const client = new StellarRouteClient({ baseUrl: API_URL });

  console.log('1. Fetching ranked routes for 1 XLM -> USDC...');
  const routesResponse = await client.getRankedRoutes('native', USDC_TESTNET, 1);
  const bestRoute = routesResponse.routes[0];
  if (!bestRoute || bestRoute.path.length !== 1) {
    console.log('Need a single classic hop route for this example.');
    return;
  }

  const hops = bestRoute.path.map((hop) => ({
    from_asset:
      hop.from_asset.asset_type === 'native'
        ? 'native'
        : `${hop.from_asset.asset_code}:${hop.from_asset.asset_issuer}`,
    to_asset:
      hop.to_asset.asset_type === 'native'
        ? 'native'
        : `${hop.to_asset.asset_code}:${hop.to_asset.asset_issuer}`,
    source: hop.source,
    fee_bps: hop.fee_bps,
    price: hop.price,
  }));

  console.log('\n2. prepareSwap...');
  const prepared = await client.prepareSwap({
    route: { hops },
    amount: '1',
    sender: SENDER_PUBLIC_KEY,
    slippage_bps: 50,
  });
  console.log(
    `quote_id=${prepared.quote_id} mode=${prepared.execution_mode} out=${prepared.expected_output} network=${prepared.network_passphrase}`,
  );

  console.log('\n3. executeSwap (sign once, ambiguous submit retries reuse body)...');
  try {
    const result = await client.executeSwap({
      route: { hops },
      amount: '1',
      sender: SENDER_PUBLIC_KEY,
      slippage_bps: 50,
      // Required: current wallet/app passphrase (or async getter). Mismatch →
      // typed network_mismatch before sign/submit.
      networkPassphrase: prepared.network_passphrase,
      signTransaction: async (xdrBase64) => {
        // Freighter: signTransaction(xdrBase64, { networkPassphrase: prepared.network_passphrase }).
        console.log('Unsigned XDR received (first 40 chars):', xdrBase64.slice(0, 40) + '…');
        console.log('Prepare network_passphrase:', prepared.network_passphrase);
        throw new Error(
          'Wire Freighter signing here — refusing to submit an unsigned envelope',
        );
      },
    });

    const confirmed = await client.confirmSwap(result.tx_hash, {
      horizonUrl: HORIZON_URL,
    });
    console.log(`confirmed=${confirmed.successful} url=${confirmed.horizon_url}`);
  } catch (error) {
    if (isStellarRouteApiError(error)) {
      console.error(`[${error.code}] ${error.message}`, error.details);
    } else {
      console.error(error);
    }
  }
}

main().catch(console.error);
