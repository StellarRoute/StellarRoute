import { API_PROXY_ENABLED } from '@/lib/constants';
import { normalizeAppNetwork, type AppNetwork } from '@/lib/network-policy';
import type { WalletNetwork } from '@/lib/wallet/types';

const HORIZON_URLS: Record<string, string> = {
  testnet: 'https://horizon-testnet.stellar.org',
  mainnet: 'https://horizon.stellar.org',
};

export const NETWORK_PASSPHRASES: Record<string, string> = {
  testnet: 'Test SDF Network ; September 2015',
  mainnet: 'Public Global Stellar Network ; September 2015',
  futurenet: 'Test SDF Future Network ; October 2022',
};

function resolveAppNetworkKey(network: WalletNetwork | null): AppNetwork {
  return normalizeAppNetwork(network) ?? 'testnet';
}

export function getHorizonUrl(network: WalletNetwork | null): string {
  const key = resolveAppNetworkKey(network);
  return HORIZON_URLS[key];
}

export function getNetworkPassphrase(network: WalletNetwork | null): string {
  const rawKey = network ? String(network).trim().toLowerCase() : null;
  if (rawKey && NETWORK_PASSPHRASES[rawKey]) {
    return NETWORK_PASSPHRASES[rawKey];
  }
  const key = resolveAppNetworkKey(network);
  return NETWORK_PASSPHRASES[key] ?? NETWORK_PASSPHRASES.testnet;
}

function stripTrailingSlash(url: string): string {
  return url.replace(/\/$/, '');
}

/**
 * API client paths include `/api/v1/...`; strip a trailing version suffix from env URLs.
 */
export function normalizeApiOrigin(url: string): string {
  return stripTrailingSlash(url).replace(/\/api\/v1$/i, '');
}

type PublicApiEnvKey =
  | 'NEXT_PUBLIC_API_URL'
  | 'NEXT_PUBLIC_API_URL_TESTNET'
  | 'NEXT_PUBLIC_API_URL_MAINNET';

/**
 * Resolve a NEXT_PUBLIC API URL from env.
 *
 * Next.js only inlines `NEXT_PUBLIC_*` values when they are accessed via a
 * static property path (`process.env.NEXT_PUBLIC_FOO`). Dynamic lookups like
 * `process.env[key]` stay undefined in the browser bundle, which previously
 * made production fall back to `http://localhost:8080` for health/quotes.
 */
function resolveEnvApiUrl(envKey: PublicApiEnvKey): string | undefined {
  const raw =
    envKey === 'NEXT_PUBLIC_API_URL_MAINNET'
      ? process.env.NEXT_PUBLIC_API_URL_MAINNET
      : envKey === 'NEXT_PUBLIC_API_URL_TESTNET'
        ? process.env.NEXT_PUBLIC_API_URL_TESTNET
        : process.env.NEXT_PUBLIC_API_URL;
  const value = raw?.trim();
  return value ? normalizeApiOrigin(value) : undefined;
}

/**
 * Resolve the StellarRoute API origin for the selected Stellar network (no `/api/v1` suffix).
 * Per-network env vars take precedence; falls back to shared NEXT_PUBLIC_API_URL / proxy.
 */
export function getApiBaseUrl(network: WalletNetwork | null): string {
  if (API_PROXY_ENABLED) {
    return '';
  }

  const appNetwork = resolveAppNetworkKey(network);
  const perNetworkUrl =
    appNetwork === 'mainnet'
      ? resolveEnvApiUrl('NEXT_PUBLIC_API_URL_MAINNET')
      : resolveEnvApiUrl('NEXT_PUBLIC_API_URL_TESTNET');

  if (perNetworkUrl) {
    return perNetworkUrl;
  }

  const sharedUrl = resolveEnvApiUrl('NEXT_PUBLIC_API_URL');
  if (sharedUrl) {
    return sharedUrl;
  }

  return 'http://localhost:8080';
}
