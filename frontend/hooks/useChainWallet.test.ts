import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clearAdaptersForTests, registerAdapter } from '@/lib/wallet/adapters';
import type { ChainWalletAdapter } from '@/lib/wallet/adapters';
import { useChainWallet } from './useChainWallet';

function createMockEvmAdapter(
  overrides?: Partial<ChainWalletAdapter>
): ChainWalletAdapter {
  return {
    id: 'evm-injected',
    label: 'EVM Wallet',
    chainFamily: 'evm',
    detectInstalled: vi.fn(async () => true),
    connect: vi.fn(async () => ({
      adapterId: 'evm-injected',
      chainFamily: 'evm' as const,
      account: { address: '0xabc' },
      network: 'eip155:11155111' as const,
      isConnected: true,
    })),
    disconnect: vi.fn(async () => undefined),
    getSession: vi.fn(async () => null),
    getNetwork: vi.fn(async () => ({
      network: 'eip155:11155111' as const,
      matchesExpected: true,
      expected: 'eip155:11155111' as const,
    })),
    signMessage: vi.fn(async () => ({
      signature: '0xsig',
      address: '0xabc',
    })),
    signTransaction: vi.fn(async () => ({
      kind: 'evm_transaction' as const,
      signedTransaction: '0xsigned',
    })),
    sendTransaction: vi.fn(async () => ({
      kind: 'evm_transaction' as const,
      hash: '0xhash',
    })),
    checkCapabilities: vi.fn(async () => ({
      checkedAt: Date.now(),
      statuses: [],
    })),
    getExecutionSupport: vi.fn(() => ({
      kind: 'unsupported' as const,
      code: 'no_backend_route' as const,
      message: 'no route',
    })),
    ...overrides,
  };
}

describe('useChainWallet', () => {
  beforeEach(() => {
    clearAdaptersForTests();
  });

  afterEach(() => {
    clearAdaptersForTests();
  });

  it('connects, exposes account/network state, and disconnects', async () => {
    registerAdapter(createMockEvmAdapter());

    const { result } = renderHook(() =>
      useChainWallet({
        chainFamily: 'evm',
        expectedNetwork: 'eip155:11155111',
      })
    );

    await act(async () => {
      await result.current.refreshWallets();
    });

    expect(result.current.availableWallets[0]?.installed).toBe(true);

    await act(async () => {
      await result.current.connect('evm-injected');
    });

    expect(result.current.isConnected).toBe(true);
    expect(result.current.address).toBe('0xabc');
    expect(result.current.networkMismatch).toBe(false);
    expect(result.current.executionSupport?.code).toBe('no_backend_route');

    await act(async () => {
      await result.current.disconnect();
    });

    expect(result.current.isConnected).toBe(false);
    expect(result.current.address).toBeNull();
  });

  it('blocks sign/send/message when network mismatches', async () => {
    registerAdapter(
      createMockEvmAdapter({
        getNetwork: vi.fn(async () => ({
          network: 'eip155:1' as const,
          matchesExpected: false,
          expected: 'eip155:11155111' as const,
        })),
      })
    );

    const { result } = renderHook(() =>
      useChainWallet({ expectedNetwork: 'eip155:11155111' })
    );

    await act(async () => {
      await result.current.connect('evm-injected');
    });

    expect(result.current.networkMismatch).toBe(true);
    expect(result.current.executionSupport?.code).toBe('network_mismatch');

    await expect(
      result.current.signTransaction({
        kind: 'evm_transaction',
        transaction: { to: '0x1' },
      })
    ).rejects.toThrow(/network does not match/i);

    await expect(
      result.current.signMessage({ kind: 'message', message: 'hi' })
    ).rejects.toThrow(/network does not match/i);
  });
});
