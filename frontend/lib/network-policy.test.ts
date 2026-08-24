import { afterEach, describe, expect, it } from 'vitest';

import {
  getAllowedNetworks,
  getDefaultNetwork,
  isNetworkAllowed,
  loadPersistedNetwork,
  NETWORK_STORAGE_KEY,
  normalizeAppNetwork,
  persistNetwork,
  resolveInitialNetwork,
} from '@/lib/network-policy';
import { getApiBaseUrl, getHorizonUrl } from '@/lib/network-endpoints';

const ENV_KEYS = [
  'NEXT_PUBLIC_MAINNET_LIMITED',
  'NEXT_PUBLIC_DEFAULT_NETWORK',
  'NEXT_PUBLIC_API_URL',
  'NEXT_PUBLIC_API_URL_TESTNET',
  'NEXT_PUBLIC_API_URL_MAINNET',
  'NEXT_PUBLIC_API_PROXY',
] as const;

function clearEnv(): void {
  for (const key of ENV_KEYS) {
    delete process.env[key];
  }
}

describe('network policy', () => {
  afterEach(() => {
    clearEnv();
    window.localStorage.clear();
  });

  it('allows only testnet when mainnet-limited flag is off', () => {
    delete process.env.NEXT_PUBLIC_MAINNET_LIMITED;

    expect(getAllowedNetworks()).toEqual(['testnet']);
    expect(isNetworkAllowed('testnet')).toBe(true);
    expect(isNetworkAllowed('mainnet')).toBe(false);
  });

  it('allows mainnet when mainnet-limited flag is enabled', () => {
    process.env.NEXT_PUBLIC_MAINNET_LIMITED = 'true';

    expect(getAllowedNetworks()).toEqual(['testnet', 'mainnet']);
    expect(isNetworkAllowed('mainnet')).toBe(true);
  });

  it('defaults to testnet unless configured default is allowed', () => {
    expect(getDefaultNetwork()).toBe('testnet');

    process.env.NEXT_PUBLIC_MAINNET_LIMITED = 'true';
    process.env.NEXT_PUBLIC_DEFAULT_NETWORK = 'mainnet';
    expect(getDefaultNetwork()).toBe('mainnet');
  });

  it('persists and loads selected network', () => {
    persistNetwork('testnet');
    expect(window.localStorage.getItem(NETWORK_STORAGE_KEY)).toBe('testnet');
    expect(loadPersistedNetwork()).toBe('testnet');
  });

  it('does not persist disallowed mainnet when flag is off', () => {
    persistNetwork('mainnet');
    expect(window.localStorage.getItem(NETWORK_STORAGE_KEY)).toBeNull();
  });

  it('clamps initial network to allowed values', () => {
    process.env.NEXT_PUBLIC_MAINNET_LIMITED = 'true';
    expect(resolveInitialNetwork('mainnet')).toBe('mainnet');
    expect(resolveInitialNetwork('futurenet', 'testnet')).toBe('testnet');
  });

  it('normalizes Freighter-style wallet network labels', () => {
    expect(normalizeAppNetwork('PUBLIC')).toBe('mainnet');
    expect(normalizeAppNetwork('TESTNET')).toBe('testnet');
    expect(normalizeAppNetwork('Public Global Stellar Network ; September 2015')).toBe(
      'mainnet',
    );
    expect(normalizeAppNetwork('Test SDF Network ; September 2015')).toBe('testnet');
    expect(normalizeAppNetwork('futurenet')).toBeNull();
  });
});

describe('network endpoints', () => {
  afterEach(() => {
    clearEnv();
  });

  it('returns horizon URLs by network', () => {
    expect(getHorizonUrl('testnet')).toBe('https://horizon-testnet.stellar.org');
    expect(getHorizonUrl('mainnet')).toBe('https://horizon.stellar.org');
    expect(getHorizonUrl('PUBLIC')).toBe('https://horizon.stellar.org');
  });

  it('prefers per-network API URLs when configured', () => {
    process.env.NEXT_PUBLIC_API_URL_TESTNET = 'https://api-test.example.com/api/v1';
    process.env.NEXT_PUBLIC_API_URL_MAINNET = 'https://api-main.example.com/api/v1';

    expect(getApiBaseUrl('testnet')).toBe('https://api-test.example.com');
    expect(getApiBaseUrl('mainnet')).toBe('https://api-main.example.com');
  });

  it('accepts API origins without a trailing /api/v1 suffix', () => {
    process.env.NEXT_PUBLIC_API_URL_TESTNET = 'https://api-test.example.com';

    expect(getApiBaseUrl('testnet')).toBe('https://api-test.example.com');
  });

  it('falls back to shared NEXT_PUBLIC_API_URL', () => {
    process.env.NEXT_PUBLIC_API_URL = 'https://api-shared.example.com/api/v1';

    expect(getApiBaseUrl('testnet')).toBe('https://api-shared.example.com');
  });
});
