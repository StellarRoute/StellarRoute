import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import {
  clearAdaptersForTests,
  ensureDefaultAdapters,
  getAdapter,
  listAdapters,
  listAvailableChainWallets,
  normalizeBitcoinNetwork,
  normalizeTronNetwork,
} from './index';
import type { UnisatProvider } from './bitcoin/types';
import type { OkxBitcoinProvider } from './bitcoin/types';
import type { TronLinkProvider, TronWebLike } from './tron/types';

const BTC_ADDRESS = 'bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh';
const TRON_ADDRESS = 'TXYZopYRdj2D9XRtbG411XZZ3kM5VkAeBf';

function mockUnisat(overrides: Partial<UnisatProvider> = {}): UnisatProvider {
  return {
    requestAccounts: vi.fn().mockResolvedValue([BTC_ADDRESS]),
    getAccounts: vi.fn().mockResolvedValue([BTC_ADDRESS]),
    getNetwork: vi.fn().mockResolvedValue('livenet'),
    switchNetwork: vi.fn().mockResolvedValue(undefined),
    getPublicKey: vi.fn().mockResolvedValue('pubhex'),
    signMessage: vi.fn().mockResolvedValue('sig-msg'),
    signPsbt: vi.fn().mockResolvedValue('signed-psbt-hex'),
    ...overrides,
  };
}

function mockOkx(overrides: Partial<OkxBitcoinProvider> = {}): OkxBitcoinProvider {
  return {
    connect: vi.fn().mockResolvedValue({ address: BTC_ADDRESS, publicKey: 'okx-pub' }),
    getAccounts: vi.fn().mockResolvedValue([BTC_ADDRESS]),
    getNetwork: vi.fn().mockResolvedValue('testnet'),
    switchNetwork: vi.fn().mockResolvedValue(undefined),
    getPublicKey: vi.fn().mockResolvedValue('okx-pub'),
    signMessage: vi.fn().mockResolvedValue('okx-sig'),
    signPsbt: vi.fn().mockResolvedValue('okx-signed-psbt'),
    ...overrides,
  };
}

function mockTronWeb(overrides: Partial<TronWebLike> = {}): TronWebLike {
  return {
    ready: true,
    defaultAddress: { base58: TRON_ADDRESS, hex: '41abc' },
    fullNode: { host: 'https://api.trongrid.io' },
    trx: {
      sign: vi.fn().mockResolvedValue({ txID: 'signed-tx' }),
      signMessageV2: vi.fn().mockResolvedValue('tron-sig'),
    },
    ...overrides,
  };
}

describe('adapter registry', () => {
  beforeEach(() => {
    clearAdaptersForTests();
  });

  afterEach(() => {
    delete window.unisat;
    delete window.okxwallet;
    delete window.tronLink;
    delete window.tronWeb;
    clearAdaptersForTests();
  });

  it('registers UniSat, OKX Bitcoin, and TronLink among defaults', () => {
    ensureDefaultAdapters();
    const ids = listAdapters().map((a) => a.id);
    expect(ids).toEqual(
      expect.arrayContaining(['okx-bitcoin', 'tronlink', 'unisat'])
    );
    expect(listAdapters('bitcoin').map((a) => a.id).sort()).toEqual([
      'okx-bitcoin',
      'unisat',
    ]);
    expect(listAdapters('tron').map((a) => a.id)).toEqual(['tronlink']);
  });

  it('lists installed wallets via safe detection', async () => {
    window.unisat = mockUnisat();
    window.tronLink = {
      ready: true,
      tronWeb: mockTronWeb(),
      request: vi.fn().mockResolvedValue({ code: 200 }),
    };

    const wallets = await listAvailableChainWallets();
    const byId = Object.fromEntries(wallets.map((w) => [w.id, w]));

    expect(byId.unisat?.installed).toBe(true);
    expect(byId.tronlink?.installed).toBe(true);
    expect(byId['okx-bitcoin']?.installed).toBe(false);
  });
});

describe('network normalization', () => {
  it('normalizes Bitcoin networks', () => {
    expect(normalizeBitcoinNetwork('livenet')).toBe('bitcoin:mainnet');
    expect(normalizeBitcoinNetwork('testnet')).toBe('bitcoin:testnet');
    expect(normalizeBitcoinNetwork('signet')).toBe('bitcoin:signet');
  });

  it('normalizes TRON hosts', () => {
    expect(normalizeTronNetwork('https://api.trongrid.io')).toBe('tron:mainnet');
    expect(normalizeTronNetwork('https://nile.trongrid.io')).toBe('tron:nile');
    expect(normalizeTronNetwork('https://api.shasta.trongrid.io')).toBe(
      'tron:shasta'
    );
  });
});

describe('UniSat adapter', () => {
  beforeEach(() => {
    clearAdaptersForTests();
    ensureDefaultAdapters();
  });

  afterEach(() => {
    delete window.unisat;
    clearAdaptersForTests();
  });

  it('connects, detects network mismatch softly, and signs via the wallet only', async () => {
    const provider = mockUnisat({
      getNetwork: vi.fn().mockResolvedValue('testnet'),
    });
    window.unisat = provider;

    const adapter = getAdapter('unisat');
    expect(adapter).toBeDefined();

    const session = await adapter!.connect('bitcoin:testnet');
    expect(session.account.address).toBe(BTC_ADDRESS);
    expect(session.network).toBe('bitcoin:testnet');
    expect(provider.requestAccounts).toHaveBeenCalled();

    // Soft connect: mismatch does not throw; hook/getNetwork flags it.
    const mismatched = await adapter!.connect('bitcoin:mainnet');
    expect(mismatched.network).toBe('bitcoin:testnet');
    const info = await adapter!.getNetwork('bitcoin:mainnet');
    expect(info.matchesExpected).toBe(false);
    expect(adapter!.getExecutionSupport().code).toBe('network_mismatch');

    provider.getNetwork = vi.fn().mockResolvedValue('livenet');
    // After matching mainnet:
    const mainSession = await adapter!.connect('bitcoin:mainnet');
    expect(mainSession.network).toBe('bitcoin:mainnet');

    const msg = await adapter!.signMessage({
      kind: 'message',
      message: 'hello',
    });
    expect(msg.signature).toBe('sig-msg');
    expect(provider.signMessage).toHaveBeenCalledWith('hello', 'ecdsa');

    const tx = await adapter!.signTransaction({
      kind: 'bitcoin_psbt',
      psbt: 'deadbeef',
      format: 'hex',
    });
    expect(tx).toEqual({
      kind: 'bitcoin_psbt',
      psbt: 'signed-psbt-hex',
      format: 'hex',
    });
    expect(provider.signPsbt).toHaveBeenCalled();
  });

  it('rethrows user rejection when UniSat network switch is denied', async () => {
    window.unisat = mockUnisat({
      getNetwork: vi.fn().mockResolvedValue('livenet'),
      switchNetwork: vi
        .fn()
        .mockRejectedValue(Object.assign(new Error('User rejected'), { code: 4001 })),
    });
    const adapter = getAdapter('unisat')!;
    await expect(adapter.connect('bitcoin:testnet')).rejects.toMatchObject({
      code: 'user_rejected',
    });
  });

  it('reports not_connected execution support before connect', () => {
    const adapter = getAdapter('unisat')!;
    expect(
      adapter.getExecutionSupport({
        sourceChain: 'bitcoin',
        destinationChain: 'bitcoin',
      }).code
    ).toBe('not_connected');
  });

  it('never signs when extension is missing', async () => {
    const adapter = getAdapter('unisat')!;
    await expect(adapter.connect()).rejects.toMatchObject({
      code: 'not_installed',
    });
  });

  it('reports capability denial on network mismatch', async () => {
    window.unisat = mockUnisat({
      getNetwork: vi.fn().mockResolvedValue('testnet'),
    });
    const adapter = getAdapter('unisat')!;
    await adapter.connect('bitcoin:testnet');

    const caps = await adapter.checkCapabilities('bitcoin:mainnet');
    const viewNet = caps.statuses.find((s) => s.capability === 'view_network');
    const signTx = caps.statuses.find((s) => s.capability === 'sign_transaction');
    expect(viewNet?.allowed).toBe(false);
    expect(signTx?.allowed).toBe(false);
  });

  it('maps user rejection on sign', async () => {
    window.unisat = mockUnisat({
      signMessage: vi.fn().mockRejectedValue(new Error('User rejected the request')),
    });
    const adapter = getAdapter('unisat')!;
    await adapter.connect();
    await expect(
      adapter.signMessage({ kind: 'message', message: 'x' })
    ).rejects.toMatchObject({ code: 'user_rejected' });
  });

  it('disconnect is a no-op that does not touch keys', async () => {
    window.unisat = mockUnisat();
    const adapter = getAdapter('unisat')!;
    await adapter.connect();
    await expect(adapter.disconnect()).resolves.toBeUndefined();
  });
});

describe('OKX Bitcoin adapter', () => {
  beforeEach(() => {
    clearAdaptersForTests();
    ensureDefaultAdapters();
  });

  afterEach(() => {
    delete window.okxwallet;
    clearAdaptersForTests();
  });

  it('connects through okxwallet.bitcoin and signs PSBTs', async () => {
    window.okxwallet = { bitcoin: mockOkx() };
    const adapter = getAdapter('okx-bitcoin')!;

    const session = await adapter.connect('bitcoin:testnet');
    expect(session.account.address).toBe(BTC_ADDRESS);
    expect(session.network).toBe('bitcoin:testnet');

    const signed = await adapter.signTransaction({
      kind: 'bitcoin_psbt',
      psbt: 'abc123',
      format: 'hex',
    });
    expect(signed.kind).toBe('bitcoin_psbt');
    if (signed.kind === 'bitcoin_psbt') {
      expect(signed.psbt).toBe('okx-signed-psbt');
    }
  });
});

describe('TronLink adapter', () => {
  beforeEach(() => {
    clearAdaptersForTests();
    ensureDefaultAdapters();
  });

  afterEach(() => {
    delete window.tronLink;
    delete window.tronWeb;
    clearAdaptersForTests();
  });

  it('connects, detects Nile vs mainnet softly, and signs only via TronWeb', async () => {
    const tronWeb = mockTronWeb({
      fullNode: { host: 'https://nile.trongrid.io' },
    });
    const request = vi.fn().mockResolvedValue({ code: 200 });
    window.tronLink = {
      ready: true,
      request,
      tronWeb,
    } satisfies TronLinkProvider;

    const adapter = getAdapter('tronlink')!;
    const session = await adapter.connect('tron:nile');
    expect(request).toHaveBeenCalledWith({ method: 'tron_requestAccounts' });
    expect(session.account.address).toBe(TRON_ADDRESS);
    expect(session.network).toBe('tron:nile');

    const mismatched = await adapter.connect('tron:mainnet');
    expect(mismatched.network).toBe('tron:nile');
    expect((await adapter.getNetwork('tron:mainnet')).matchesExpected).toBe(
      false
    );

    // Align expected network for signing gates.
    await adapter.getNetwork('tron:nile');

    const msg = await adapter.signMessage({
      kind: 'message',
      message: 'gm',
    });
    expect(msg.signature).toBe('tron-sig');

    const tx = await adapter.signTransaction({
      kind: 'tron_transaction',
      transaction: { raw_data: {} },
    });
    expect(tx.kind).toBe('tron_transaction');
    expect(tronWeb.trx?.sign).toHaveBeenCalled();
  });

  it('rejects wrong payload kinds', async () => {
    window.tronLink = {
      ready: true,
      tronWeb: mockTronWeb(),
      request: vi.fn().mockResolvedValue({ code: 200 }),
    };
    const adapter = getAdapter('tronlink')!;
    await adapter.connect();
    await expect(
      adapter.signTransaction({
        kind: 'bitcoin_psbt',
        psbt: 'ff',
        format: 'hex',
      })
    ).rejects.toMatchObject({ code: 'invalid_request' });
  });

  it('exposes unsupported swap execution support after connect', async () => {
    window.tronLink = {
      ready: true,
      tronWeb: mockTronWeb(),
      request: vi.fn().mockResolvedValue({ code: 200 }),
    };
    const adapter = getAdapter('tronlink')!;
    await adapter.connect('tron:mainnet');
    const support = adapter.getExecutionSupport({
      sourceChain: 'tron',
      destinationChain: 'stellar',
    });
    expect(support.kind).toBe('unsupported');
    expect(support.code).toBe('no_backend_route');
  });
});
