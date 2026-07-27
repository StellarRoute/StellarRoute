import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  checkWalletCapabilities,
  connectWallet,
  getAvailableWallets,
  refreshWalletSession,
  signTransactionWithWallet,
} from './index';

const TEST_PASSPHRASE = 'Test SDF Network ; September 2015';
const PUBLIC_PASSPHRASE = 'Public Global Stellar Network ; September 2015';
const MOCK_PUBLIC_KEY =
  'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ';
const MOCK_XDR = 'AAAAAgAAAABmockTransactionXdrBase64';

describe('wallet availability', () => {
  it('includes Albedo as a browser-hosted intent wallet', async () => {
    const wallets = await getAvailableWallets();
    const albedo = wallets.find((wallet) => wallet.id === 'albedo');

    expect(albedo).toEqual({
      id: 'albedo',
      label: 'Albedo',
      installed: true,
    });
  });
});

describe('connectWallet - Albedo', () => {
  const mockPublicKey = vi.fn();

  beforeEach(() => {
    mockPublicKey.mockReset();
    window.albedo = {
      publicKey: mockPublicKey,
      tx: vi.fn(),
    };
  });

  afterEach(() => {
    delete window.albedo;
  });

  it('connects with the public key intent', async () => {
    mockPublicKey.mockResolvedValue({ pubkey: MOCK_PUBLIC_KEY });

    const session = await connectWallet('albedo');

    expect(session).toMatchObject({
      walletId: 'albedo',
      address: MOCK_PUBLIC_KEY,
      network: 'testnet',
      isConnected: true,
    });
    expect(mockPublicKey).toHaveBeenCalledOnce();
  });

  it('refreshes via the same public key intent', async () => {
    mockPublicKey.mockResolvedValue({ publicKey: MOCK_PUBLIC_KEY });

    const session = await refreshWalletSession('albedo');

    expect(session.address).toBe(MOCK_PUBLIC_KEY);
    expect(mockPublicKey).toHaveBeenCalledOnce();
  });
});

describe('signTransactionWithWallet - Albedo', () => {
  const mockTx = vi.fn();

  beforeEach(() => {
    mockTx.mockReset();
    window.albedo = {
      publicKey: vi.fn(),
      tx: mockTx,
    };
  });

  afterEach(() => {
    delete window.albedo;
  });

  it('returns signed XDR on testnet with public key', async () => {
    mockTx.mockResolvedValue({ signed_envelope_xdr: 'signed_xdr' });

    const result = await signTransactionWithWallet(
      MOCK_XDR,
      'albedo',
      TEST_PASSPHRASE,
      MOCK_PUBLIC_KEY
    );

    expect(result).toBe('signed_xdr');
    expect(mockTx).toHaveBeenCalledWith({
      xdr: MOCK_XDR,
      network: 'testnet',
      pubkey: MOCK_PUBLIC_KEY,
    });
  });

  it('maps public network passphrase to Albedo public network', async () => {
    mockTx.mockResolvedValue({ xdr: 'signed_public_xdr' });

    await signTransactionWithWallet(MOCK_XDR, 'albedo', PUBLIC_PASSPHRASE);

    expect(mockTx).toHaveBeenCalledWith({
      xdr: MOCK_XDR,
      network: 'public',
      pubkey: undefined,
    });
  });

  it('throws user-facing message when user cancels', async () => {
    mockTx.mockRejectedValue(new Error('User rejected transaction'));

    await expect(
      signTransactionWithWallet(MOCK_XDR, 'albedo', TEST_PASSPHRASE)
    ).rejects.toThrow('User declined transaction signing');
  });
});

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

describe('checkWalletCapabilities - Albedo', () => {
  it('allows sign_transaction on mainnet', async () => {
    const caps = await checkWalletCapabilities('albedo', 'mainnet');
    const signCap = caps.statuses.find(
      (s) => s.capability === 'sign_transaction'
    );

    expect(signCap?.allowed).toBe(true);
    expect(signCap?.reason).toBeUndefined();
  });

  it('denies unsupported networks with a resolution', async () => {
    const caps = await checkWalletCapabilities('albedo', 'futurenet');
    const signCap = caps.statuses.find(
      (s) => s.capability === 'sign_transaction'
    );

    expect(signCap?.allowed).toBe(false);
    expect(signCap?.resolution).toBe('Switch app to testnet or mainnet');
  });
});
