/**
 * Embed a StellarRoute price quote in ~20 lines.
 *
 * Staging API: https://52.206.173.91.sslip.io
 * Swap to a different base URL for production: https://api.stellarroute.io
 *
 * Run:
 *   npx ts-node --esm sdk-js/examples/embed-quote.ts
 */

import { StellarRouteClient } from '../src/index.js';

const STAGING_URL = 'https://52.206.173.91.sslip.io';
const USDC_ISSUER = 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN';

const client = new StellarRouteClient(STAGING_URL);

const quote = await client.getQuote('native', `USDC:${USDC_ISSUER}`, 100);

console.log(`XLM → USDC  |  100 XLM = ${quote.total} USDC  |  price ${quote.price}`);
