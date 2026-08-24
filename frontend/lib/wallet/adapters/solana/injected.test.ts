import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clearAdaptersForTests, getAdapter } from '../registry';
import { createInjectedSolanaAdapter } from './injected';

type MockSolana = {
  isPhantom?: boolean;
  isConnected?: boolean;
  publicKey?: { toString: () => string; toBase58?: () => string } | null;
  connect: ReturnType<typeof vi.fn>;
  disconnect?: ReturnType<typeof vi.fn>;
  signMessage?: ReturnType<typeof vi.fn>;
  signTransaction?: ReturnType<typeof vi.fn>;
  signAndSendTransaction?: ReturnType<typeof vi.fn>;
  network?: string;
};

function installWallet(wallet: MockSolana | null) {
  const win = window as unknown as Record<string, unknown>;
  if (wallet) {
    win.solana = wallet;
    win.phantom = { solana: wallet };
  } else {
    delete win.solana;
    delete win.phantom;
  }
}

describe('injected Solana adapter', () => {
  beforeEach(() => {
    clearAdaptersForTests();
    installWallet(null);
  });

  afterEach(() => {
    installWallet(null);
    clearAdaptersForTests();
  });

  it('detects missing provider as not installed', async () => {
    const adapter = createInjectedSolanaAdapter();
    await expect(adapter.detectInstalled()).resolves.toBe(false);
  });

  it('connects through phantom-style connect()', async () => {
    const publicKey = {
      toString: () => 'So11111111111111111111111111111111111111112',
      toBase58: () => 'So11111111111111111111111111111111111111112',
    };
    installWallet({
      isPhantom: true,
      isConnected: true,
      publicKey,
      network: 'devnet',
      connect: vi.fn(async () => ({ publicKey })),
      disconnect: vi.fn(async () => undefined),
      signTransaction: vi.fn(),
    });

    const adapter = createInjectedSolanaAdapter();
    const session = await adapter.connect('solana:devnet');

    expect(session).toMatchObject({
      adapterId: 'solana-injected',
      chainFamily: 'solana',
      account: { address: 'So11111111111111111111111111111111111111112' },
      network: 'solana:devnet',
      isConnected: true,
    });
  });

  it('rejects raw serialized bytes with unsupported_capability', async () => {
    const publicKey = {
      toString: () => 'So11111111111111111111111111111111111111112',
    };
    installWallet({
      isPhantom: true,
      isConnected: true,
      publicKey,
      network: 'devnet',
      connect: vi.fn(async () => ({ publicKey })),
      signAndSendTransaction: vi.fn(async () => ({ signature: 'sig123' })),
      signTransaction: vi.fn(),
    });

    const adapter = createInjectedSolanaAdapter();
    await adapter.connect('solana:devnet');
    await expect(
      adapter.sendTransaction?.({
        kind: 'solana_transaction',
        transaction: new Uint8Array([1, 2, 3, 4]),
        encoding: 'bytes',
      })
    ).rejects.toMatchObject({ code: 'unsupported_capability' });
  });

  it('signs wallet-compatible Transaction objects', async () => {
    const publicKey = {
      toString: () => 'So11111111111111111111111111111111111111112',
    };
    const signedBytes = new Uint8Array([9, 9, 9]);
    const signTransaction = vi.fn(async () => ({
      serialize: () => signedBytes,
    }));
    installWallet({
      isPhantom: true,
      isConnected: true,
      publicKey,
      network: 'devnet',
      connect: vi.fn(async () => ({ publicKey })),
      signTransaction,
    });

    const adapter = createInjectedSolanaAdapter();
    await adapter.connect('solana:devnet');
    const tx = { serialize: () => new Uint8Array([1, 2, 3]) };
    const result = await adapter.signTransaction({
      kind: 'solana_transaction',
      transaction: tx,
    });
    expect(result.kind).toBe('solana_transaction');
    expect(signTransaction).toHaveBeenCalledWith(tx);
  });

  it('soft-connects when wallet cluster differs', async () => {
    const publicKey = {
      toString: () => 'So11111111111111111111111111111111111111112',
    };
    installWallet({
      isPhantom: true,
      isConnected: true,
      publicKey,
      network: 'mainnet-beta',
      connect: vi.fn(async () => ({ publicKey })),
      signTransaction: vi.fn(),
    });

    const adapter = createInjectedSolanaAdapter();
    const session = await adapter.connect('solana:devnet');
    expect(session.isConnected).toBe(true);
    const info = await adapter.getNetwork('solana:devnet');
    expect(info.matchesExpected).toBe(false);
    expect(info.network).toBe('solana:mainnet');
    expect(adapter.getExecutionSupport().code).toBe('network_mismatch');
  });

  it('reports wallet_capability_missing when connected wallet cannot sign', async () => {
    const publicKey = {
      toString: () => 'So11111111111111111111111111111111111111112',
    };
    installWallet({
      isPhantom: true,
      isConnected: true,
      publicKey,
      network: 'devnet',
      connect: vi.fn(async () => ({ publicKey })),
      // no signTransaction
    });

    const adapter = createInjectedSolanaAdapter();
    await adapter.connect('solana:devnet');
    expect(adapter.getExecutionSupport().code).toBe('wallet_capability_missing');
  });

  it('registers in the default adapter registry', () => {
    expect(getAdapter('solana-injected')?.label).toBe('Solana Wallet');
  });
});
