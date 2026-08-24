import { describe, expect, it, vi, beforeEach } from 'vitest';
import { createStellarWalletAdapter } from './legacy';

vi.mock('../../index', () => ({
  signTransactionWithWallet: vi.fn(),
  connectWallet: vi.fn(),
  disconnectWallet: vi.fn(),
  refreshWalletSession: vi.fn(),
  checkWalletCapabilities: vi.fn().mockResolvedValue({ checkedAt: Date.now(), statuses: [] }),
  getAvailableWallets: vi.fn().mockResolvedValue([]),
}));

import { signTransactionWithWallet } from '../../index';

describe('stellar legacy adapters', () => {
  beforeEach(() => {
    vi.mocked(signTransactionWithWallet).mockReset();
  });

  for (const walletId of ['freighter', 'xbull', 'albedo', 'lobstr'] as const) {
    it(`${walletId} normalizes signedXdr result`, async () => {
      vi.mocked(signTransactionWithWallet).mockResolvedValue('signed-xdr-envelope');
      const adapter = createStellarWalletAdapter(walletId);
      const result = await adapter.signTransaction({
        kind: 'stellar_xdr',
        xdr: 'AAAAxdr',
        networkPassphrase: 'Test SDF Network ; September 2015',
      });
      expect(result).toEqual({ kind: 'stellar_xdr', signedXdr: 'signed-xdr-envelope' });
    });

    it(`${walletId} propagates rejection errors`, async () => {
      vi.mocked(signTransactionWithWallet).mockRejectedValue(
        new Error('User declined transaction signing'),
      );
      const adapter = createStellarWalletAdapter(walletId);
      await expect(
        adapter.signTransaction({
          kind: 'stellar_xdr',
          xdr: 'AAAAxdr',
          networkPassphrase: 'Test SDF Network ; September 2015',
        }),
      ).rejects.toThrow(/declined/i);
    });
  }
});
