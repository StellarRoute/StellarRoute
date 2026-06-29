import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { signTransactionWithWallet, checkWalletCapabilities } from './index';

const TEST_PASSPHRASE = 'Test SDF Network ; September 2015';
const MOCK_PUBLIC_KEY =
  'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ';
const MOCK_XDR = 'AAAAAgAAAABmockTransactionXdrBase64';

describe('signTransactionWithWallet - xBull', () => {
  const mockSign = vi.fn();

  beforeEach(() => {
    mockSign.mockReset();
    (window as unknown as Record<string, unknown>).xbull = {
      connect: vi.fn(),
      sign: mockSign,
    };
  });

  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).xbull;
  });

  it('returns signed XDR on success with network and publicKey', async () => {
    mockSign.mockResolvedValue('signed_xdr');

    const result = await signTransactionWithWallet(
      MOCK_XDR,
      'xbull',
      TEST_PASSPHRASE,
      MOCK_PUBLIC_KEY
    );

    expect(result).toBe('signed_xdr');
    expect(mockSign).toHaveBeenCalledWith({
      xdr: MOCK_XDR,
      network: 'testnet',
      publicKey: MOCK_PUBLIC_KEY,
    });
  });

  it('throws user-facing message when user cancels', async () => {
    mockSign.mockRejectedValue(new Error('User cancelled signing'));

    await expect(
      signTransactionWithWallet(
        MOCK_XDR,
        'xbull',
        TEST_PASSPHRASE,
        MOCK_PUBLIC_KEY
      )
    ).rejects.toThrow('User declined transaction signing');
  });

  it('throws when xBull is not installed', async () => {
    delete (window as unknown as Record<string, unknown>).xbull;

    await expect(
      signTransactionWithWallet(MOCK_XDR, 'xbull', TEST_PASSPHRASE)
    ).rejects.toThrow('xBull not installed');
  });
});

describe('checkWalletCapabilities - xBull', () => {
  it('denies sign_transaction on mainnet with testnet resolution', async () => {
    const caps = await checkWalletCapabilities('xbull', 'mainnet');
    const signCap = caps.statuses.find(
      (s) => s.capability === 'sign_transaction'
    );

    expect(signCap?.allowed).toBe(false);
    expect(signCap?.reason).toBe('xBull only supports testnet');
    expect(signCap?.resolution).toBe('Switch app to testnet');
  });

  it('allows sign_transaction on testnet', async () => {
    const caps = await checkWalletCapabilities('xbull', 'testnet');
    const signCap = caps.statuses.find(
      (s) => s.capability === 'sign_transaction'
    );

    expect(signCap?.allowed).toBe(true);
    expect(signCap?.reason).toBeUndefined();
    expect(signCap?.resolution).toBeUndefined();
  });
});
