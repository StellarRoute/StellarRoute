import { renderHook } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { useCrossChainWalletRoles } from './useCrossChainWalletRoles';

const stellarWallet = {
  address: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
  walletId: 'freighter' as const,
  isConnected: true,
  isLoading: false,
  networkMismatch: false,
  connect: vi.fn(),
  disconnect: vi.fn(),
};

const evmSource = {
  address: '0xsource',
  adapterId: 'evm:source',
  isConnected: true,
  isLoading: false,
  networkMismatch: false,
  availableWallets: [{ id: 'evm:injected', label: 'EVM Wallet', installed: true }],
  connect: vi.fn(),
  disconnect: vi.fn(),
};

const evmDest = {
  address: '0xdest',
  adapterId: 'evm:dest',
  isConnected: true,
  isLoading: false,
  networkMismatch: false,
  availableWallets: [{ id: 'evm:injected', label: 'EVM Wallet', installed: true }],
  connect: vi.fn(),
  disconnect: vi.fn(),
};

const stellarMint = {
  address: 'GCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC',
  adapterId: 'stellar:mint',
  isConnected: true,
  isLoading: false,
  networkMismatch: false,
  availableWallets: [],
  connect: vi.fn(),
  disconnect: vi.fn(),
};

let evmCallIndex = 0;

vi.mock('@/components/providers/wallet-provider', () => ({
  useWallet: () => stellarWallet,
}));

vi.mock('@/hooks/useChainWallet', () => ({
  useChainWallet: (opts: { chainFamily?: string }) => {
    if (opts?.chainFamily === 'evm') {
      evmCallIndex += 1;
      return evmCallIndex === 1 ? evmSource : evmDest;
    }
    return stellarMint;
  },
}));

describe('useCrossChainWalletRoles', () => {
  beforeEach(() => {
    evmCallIndex = 0;
  });

  it('Stellar→EVM uses destination EVM adapter for mint, not source', () => {
    const { result } = renderHook(() =>
      useCrossChainWalletRoles({
        sourceChainId: 'stellar',
        destChainId: 'ethereum-sepolia',
      }),
    );
    expect(result.current.direction).toBe('stellar_to_evm');
    expect(result.current.sagaWallets.sourceStellarAdapterId).toBe('freighter');
    expect(result.current.sagaWallets.evmDestinationAdapterId).toBe('evm:dest');
    expect(result.current.sagaWallets.sourceEvmAdapterId).toBeUndefined();
    expect(result.current.destRecipientAddress).toBe('0xdest');
  });

  it('EVM→Stellar separates muxed recipient from G mint submitter', () => {
    evmCallIndex = 0;
    const muxed =
      'MAH4OLUSPDOHMFUENP2X3YUIIML7AE62ZOLHZE5X6C622WXPXLH2MAAAAAAAAAAAABCGY';
    const { result } = renderHook(() =>
      useCrossChainWalletRoles({
        sourceChainId: 'ethereum-sepolia',
        destChainId: 'stellar',
        useRecipientOverride: true,
        recipientOverride: muxed,
      }),
    );
    expect(result.current.isMuxedRecipient).toBe(true);
    expect(result.current.showMintSubmitterChip).toBe(true);
    expect(result.current.destRecipientAddress).toBe(muxed);
    expect(result.current.sagaWallets.mintSubmitterStellarAdapterId).toBe(
      'stellar:mint',
    );
  });
});
