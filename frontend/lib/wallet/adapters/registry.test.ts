import { afterEach, describe, expect, it } from 'vitest';
import {
  clearAdaptersForTests,
  ensureDefaultAdapters,
  listAdapters,
  listAvailableChainWallets,
  registerAdapter,
} from './registry';
import type { ChainWalletAdapter } from './types';

const DEFAULT_ADAPTER_IDS = [
  'albedo',
  'evm-injected',
  'evm-walletconnect',
  'freighter',
  'lobstr',
  'okx-bitcoin',
  'solana-injected',
  'tronlink',
  'unisat',
  'xbull',
].sort();

function stubAdapter(
  id: string,
  overrides?: Partial<ChainWalletAdapter>
): ChainWalletAdapter {
  return {
    id,
    label: `Stub ${id}`,
    chainFamily: 'evm',
    detectInstalled: async () => false,
    connect: async () => {
      throw new Error('not implemented');
    },
    disconnect: async () => undefined,
    getSession: async () => null,
    getNetwork: async () => ({
      network: 'eip155:1',
      matchesExpected: true,
    }),
    signMessage: async () => {
      throw new Error('not implemented');
    },
    signTransaction: async () => {
      throw new Error('not implemented');
    },
    checkCapabilities: async () => ({ checkedAt: Date.now(), statuses: [] }),
    getExecutionSupport: () => ({
      kind: 'unsupported',
      code: 'no_backend_route',
      message: 'stub',
    }),
    ...overrides,
  };
}

describe('wallet adapter registry', () => {
  afterEach(() => {
    clearAdaptersForTests();
  });

  it('registers stellar, evm, solana, bitcoin, and tron adapters by default', () => {
    ensureDefaultAdapters();
    const ids = listAdapters().map((a) => a.id).sort();
    expect(ids).toEqual(DEFAULT_ADAPTER_IDS);
  });

  it('is idempotent when ensureDefaultAdapters is called repeatedly', () => {
    ensureDefaultAdapters();
    ensureDefaultAdapters();
    ensureDefaultAdapters();
    expect(listAdapters().map((a) => a.id).sort()).toEqual(DEFAULT_ADAPTER_IDS);
  });

  it('does not clobber a pre-registered adapter with the same id', () => {
    const custom = stubAdapter('evm-injected', { label: 'Custom EVM' });
    registerAdapter(custom);
    ensureDefaultAdapters();
    expect(listAdapters('evm').map((a) => a.id).sort()).toEqual([
      'evm-injected',
      'evm-walletconnect',
    ]);
    expect(listAdapters('evm').find((a) => a.id === 'evm-injected')?.label).toBe(
      'Custom EVM'
    );
  });

  it('filters adapters by chain family', () => {
    expect(listAdapters('evm').map((a) => a.id).sort()).toEqual([
      'evm-injected',
      'evm-walletconnect',
    ]);
    expect(listAdapters('solana').map((a) => a.id)).toEqual([
      'solana-injected',
    ]);
    expect(listAdapters('bitcoin').map((a) => a.id).sort()).toEqual([
      'okx-bitcoin',
      'unisat',
    ]);
    expect(listAdapters('tron').map((a) => a.id)).toEqual(['tronlink']);
    expect(listAdapters('stellar')).toHaveLength(4);
  });

  it('lists availability without throwing when providers are absent', async () => {
    const wallets = await listAvailableChainWallets('evm');
    expect(wallets.map((w) => w.id).sort()).toEqual([
      'evm-injected',
      'evm-walletconnect',
    ]);
    expect(
      wallets.find((w) => w.id === 'evm-injected')
    ).toMatchObject({
      chainFamily: 'evm',
      installed: false,
    });
    // WalletConnect availability depends on NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID.
    expect(
      wallets.find((w) => w.id === 'evm-walletconnect')
    ).toMatchObject({
      chainFamily: 'evm',
      id: 'evm-walletconnect',
    });

    const btc = await listAvailableChainWallets('bitcoin');
    expect(btc.map((w) => w.id).sort()).toEqual(['okx-bitcoin', 'unisat']);
    expect(btc.every((w) => w.installed === false)).toBe(true);
  });
});
