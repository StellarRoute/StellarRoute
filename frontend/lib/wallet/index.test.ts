import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import * as freighter from '@stellar/freighter-api';
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

  it('includes LOBSTR in the wallet list', async () => {
    const wallets = await getAvailableWallets();
    const lobstr = wallets.find((wallet) => wallet.id === 'lobstr');

    expect(lobstr).toEqual({
      id: 'lobstr',
      label: 'LOBSTR',
      installed: false,
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

describe('signTransactionWithWallet - Freighter passphrases', () => {
  afterEach(() => {
    vi.mocked(freighter.signTransaction).mockClear();
  });

  it('passes the canonical testnet passphrase to Freighter', async () => {
    vi.mocked(freighter.signTransaction).mockResolvedValueOnce({
      signedTxXdr: 'signed_testnet_xdr',
      signerAddress: MOCK_PUBLIC_KEY,
    });

    await expect(
      signTransactionWithWallet(MOCK_XDR, 'freighter', TEST_PASSPHRASE)
    ).resolves.toBe('signed_testnet_xdr');

    expect(freighter.signTransaction).toHaveBeenCalledWith(MOCK_XDR, {
      networkPassphrase: TEST_PASSPHRASE,
    });
  });

  it('passes the canonical public network passphrase to Freighter', async () => {
    vi.mocked(freighter.signTransaction).mockResolvedValueOnce({
      signedTxXdr: 'signed_public_xdr',
      signerAddress: MOCK_PUBLIC_KEY,
    });

    await expect(
      signTransactionWithWallet(MOCK_XDR, 'freighter', PUBLIC_PASSPHRASE)
    ).resolves.toBe('signed_public_xdr');

    expect(freighter.signTransaction).toHaveBeenCalledWith(MOCK_XDR, {
      networkPassphrase: PUBLIC_PASSPHRASE,
    });
  });
});

describe('connectWallet - xBull', () => {
  const mockConnect = vi.fn();
  const mockGetNetwork = vi.fn();

  beforeEach(() => {
    mockConnect.mockReset();
    mockGetNetwork.mockReset();
    (window as unknown as Record<string, unknown>).xbull = {
      connect: mockConnect,
      sign: vi.fn(),
      getNetwork: mockGetNetwork,
    };
  });

  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).xbull;
    delete (window as unknown as Record<string, unknown>).xBullSDK;
  });

  it('returns the real public network from getNetwork for mismatch checks', async () => {
    mockConnect.mockResolvedValue({ publicKey: MOCK_PUBLIC_KEY });
    mockGetNetwork.mockResolvedValue({
      network: 'PUBLIC',
      networkPassphrase: PUBLIC_PASSPHRASE,
    });

    const session = await connectWallet('xbull');

    expect(session).toMatchObject({
      walletId: 'xbull',
      address: MOCK_PUBLIC_KEY,
      network: 'public',
      isConnected: true,
    });
    expect(mockGetNetwork).toHaveBeenCalledOnce();
  });

  it('returns testnet when xBull reports TESTNET', async () => {
    mockConnect.mockResolvedValue({ publicKey: MOCK_PUBLIC_KEY });
    mockGetNetwork.mockResolvedValue({
      network: 'TESTNET',
      networkPassphrase: TEST_PASSPHRASE,
    });

    const session = await connectWallet('xbull');

    expect(session.network).toBe('testnet');
  });

  it('reads getNetwork from xBullSDK when window.xbull is absent', async () => {
    delete (window as unknown as Record<string, unknown>).xbull;
    (window as unknown as Record<string, unknown>).xBullSDK = {
      connect: mockConnect,
      getNetwork: mockGetNetwork,
    };
    mockConnect.mockResolvedValue({ publicKey: MOCK_PUBLIC_KEY });
    mockGetNetwork.mockResolvedValue({ network: 'public' });

    const session = await connectWallet('xbull');

    expect(session.network).toBe('public');
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
  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).xbull;
  });

  it('flags network mismatch when wallet is on public and app expects testnet', async () => {
    (window as unknown as Record<string, unknown>).xbull = {
      connect: vi.fn(),
      sign: vi.fn(),
      getNetwork: vi.fn().mockResolvedValue({
        network: 'PUBLIC',
        networkPassphrase: PUBLIC_PASSPHRASE,
      }),
    };

    const caps = await checkWalletCapabilities('xbull', 'testnet');
    const netCap = caps.statuses.find((s) => s.capability === 'view_network');
    const signCap = caps.statuses.find(
      (s) => s.capability === 'sign_transaction'
    );

    expect(netCap?.allowed).toBe(false);
    expect(netCap?.reason).toMatch(/Wallet on public/i);
    expect(signCap?.allowed).toBe(false);
    expect(signCap?.reason).toBe('Network mismatch');
  });

  it('allows sign_transaction on mainnet when wallet reports public', async () => {
    (window as unknown as Record<string, unknown>).xbull = {
      connect: vi.fn(),
      sign: vi.fn(),
      getNetwork: vi.fn().mockResolvedValue({ network: 'public' }),
    };

    const caps = await checkWalletCapabilities('xbull', 'mainnet');
    const signCap = caps.statuses.find(
      (s) => s.capability === 'sign_transaction'
    );

    expect(signCap?.allowed).toBe(true);
    expect(signCap?.reason).toBeUndefined();
  });

  it('allows sign_transaction on testnet when networks match', async () => {
    (window as unknown as Record<string, unknown>).xbull = {
      connect: vi.fn(),
      sign: vi.fn(),
      getNetwork: vi.fn().mockResolvedValue({ network: 'testnet' }),
    };

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

describe('connectWallet - LOBSTR', () => {
  it('connects when the extension returns a public key', async () => {
    const lobstr = await import('@lobstrco/signer-extension-api');
    vi.mocked(lobstr.isConnected).mockResolvedValueOnce(true);
    vi.mocked(lobstr.getPublicKey).mockResolvedValueOnce(MOCK_PUBLIC_KEY);

    const session = await connectWallet('lobstr');

    expect(session).toMatchObject({
      walletId: 'lobstr',
      address: MOCK_PUBLIC_KEY,
      isConnected: true,
    });
  });

  it('throws when the extension is missing', async () => {
    const lobstr = await import('@lobstrco/signer-extension-api');
    vi.mocked(lobstr.isConnected).mockResolvedValueOnce(false);

    await expect(connectWallet('lobstr')).rejects.toThrow(
      /LOBSTR extension is not installed/
    );
  });
});

describe('signTransactionWithWallet - LOBSTR', () => {
  it('returns the signed XDR from the extension', async () => {
    const lobstr = await import('@lobstrco/signer-extension-api');
    vi.mocked(lobstr.signTransaction).mockResolvedValueOnce('SIGNED_XDR');

    await expect(
      signTransactionWithWallet(MOCK_XDR, 'lobstr', TEST_PASSPHRASE)
    ).resolves.toBe('SIGNED_XDR');
  });
});
