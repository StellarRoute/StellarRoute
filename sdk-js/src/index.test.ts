import { describe, expect, it } from 'vitest';

import {
  StellarRouteClient,
  DEFAULT_TESTNET_HORIZON_URL,
  DEFAULT_PUBLIC_HORIZON_URL,
  TESTNET_NETWORK_PASSPHRASE,
  PUBLIC_NETWORK_PASSPHRASE,
} from './index.js';
import type { StellarSdkNetwork } from './index.js';

describe('package entrypoint exports', () => {
  it('exports the StellarSdkNetwork type, usable for StellarRouteClientOptions.network', () => {
    const network: StellarSdkNetwork = 'public';
    const client = new StellarRouteClient({ network });
    expect(client).toBeInstanceOf(StellarRouteClient);
  });

  it('exports the Horizon URL and network passphrase constants', () => {
    expect(DEFAULT_TESTNET_HORIZON_URL).toBe('https://horizon-testnet.stellar.org');
    expect(DEFAULT_PUBLIC_HORIZON_URL).toBe('https://horizon.stellar.org');
    expect(TESTNET_NETWORK_PASSPHRASE).toBe('Test SDF Network ; September 2015');
    expect(PUBLIC_NETWORK_PASSPHRASE).toBe(
      'Public Global Stellar Network ; September 2015',
    );
  });
});
