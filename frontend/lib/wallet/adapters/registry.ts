/**
 * Adapter registry — Stellar wrappers, EVM, Solana, Bitcoin, and TRON.
 *
 * Defaults are registered idempotently; tests / callers may register
 * additional adapters without clobbering ids already present.
 */

import type {
  AvailableChainWallet,
  ChainFamily,
  ChainWalletAdapter,
} from './types';
import {
  createOkxBitcoinAdapter,
  createUnisatAdapter,
} from './bitcoin';
import {
  createInjectedEvmAdapter,
  createWalletConnectEvmAdapter,
} from './evm';
import { createInjectedSolanaAdapter } from './solana';
import { createAllStellarAdapters } from './stellar/legacy';
import { createTronLinkAdapter } from './tron';

const adapters = new Map<string, ChainWalletAdapter>();
let defaultsRegistered = false;

export function registerAdapter(adapter: ChainWalletAdapter): void {
  adapters.set(adapter.id, adapter);
}

export function unregisterAdapter(id: string): void {
  adapters.delete(id);
}

export function getAdapter(id: string): ChainWalletAdapter | undefined {
  ensureDefaultAdapters();
  return adapters.get(id);
}

export function listAdapters(chainFamily?: ChainFamily): ChainWalletAdapter[] {
  ensureDefaultAdapters();
  const all = Array.from(adapters.values());
  return chainFamily
    ? all.filter((a) => a.chainFamily === chainFamily)
    : all;
}

export function clearAdaptersForTests(): void {
  adapters.clear();
  defaultsRegistered = false;
}

function registerDefault(adapter: ChainWalletAdapter): void {
  // Do not clobber adapters registered by tests or callers.
  if (!adapters.has(adapter.id)) {
    adapters.set(adapter.id, adapter);
  }
}

/** Idempotent registration of all built-in chain wallet adapters. */
export function ensureDefaultAdapters(): void {
  if (defaultsRegistered) return;
  defaultsRegistered = true;

  for (const adapter of createAllStellarAdapters()) {
    registerDefault(adapter);
  }
  registerDefault(createInjectedEvmAdapter());
  registerDefault(createWalletConnectEvmAdapter());
  registerDefault(createInjectedSolanaAdapter());
  registerDefault(createUnisatAdapter());
  registerDefault(createOkxBitcoinAdapter());
  registerDefault(createTronLinkAdapter());
}

export async function listAvailableChainWallets(
  chainFamily?: ChainFamily
): Promise<AvailableChainWallet[]> {
  const selected = listAdapters(chainFamily);
  return Promise.all(
    selected.map(async (adapter) => {
      let installed = false;
      try {
        installed = await adapter.detectInstalled();
      } catch {
        installed = false;
      }
      return {
        id: adapter.id,
        label: adapter.label,
        chainFamily: adapter.chainFamily,
        installed,
        installUrl: adapter.installUrl,
      };
    })
  );
}
