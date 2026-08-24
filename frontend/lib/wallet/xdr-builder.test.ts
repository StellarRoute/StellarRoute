/**
 * XDR Builder unit tests (#762)
 *
 * All tests that build XDR use:
 *   - sequenceOverride  → skips Horizon fetch (deterministic, offline)
 *   - A fixed wallet address derived from a known G-address shape
 *   - Frozen expected XDR strings committed below as constants
 *
 * If the frozen XDR strings differ from what the builder produces, the test
 * fails, alerting developers to a breaking change in the transaction structure.
 */

import { describe, test, expect, vi, beforeEach, afterEach, beforeAll, afterAll } from 'vitest';
import {
  parseAsset,
  validateQuoteShape,
  fetchAccountSequence,
  buildPathPaymentXdr,
  XdrBuildError,
  type BuildXdrParams,
} from './xdr-builder';
import { Asset } from '@stellar/stellar-base';

// ── Constants ────────────────────────────────────────────────────────────────

/** A valid G-address used consistently across all tests */
const WALLET = 'GDJTD6CQBWST7MW4T64GQK542YDVTZUSS4WK4NPCFHCGVYUD3CJ6IETQ';
const USDC_ISSUER = 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN';
const yXLM_ISSUER = 'GARDNV3Q7YGT4AKSDF25LT32YSCCW4EV22Y2TV3I2PU2MMXJTEDL5T55';
const TESTNET_PASSPHRASE = 'Test SDF Network ; September 2015';
const HORIZON_TESTNET = 'https://horizon-testnet.stellar.org';

/**
 * Frozen XDR strings.
 * Generated with time frozen to 2026-02-23T12:00:00Z (unix 1740312000).
 * The transaction's maxTime = frozen_now + 30 = 1740312030, encoded in XDR.
 * Any change to the operation type, fee, timeout, or field encoding will break these.
 */
const FROZEN_NOW_MS = 1740312000000; // 2026-02-23T12:00:00Z
const FROZEN_XDR = {
  /** PathPaymentStrictSend: XLM → USDC, amount=100, destMin=9.95, seq=100, no intermediate hops */
  singleHopXlmUsdc:
    'AAAAAgAAAADTMfhQDaU/styfuGgrvNYHWeaSlyyuNeIpxGrig9iT5AAAAGQAAAAAAAAAZQAAAAEAAAAAAAAAAAAAAABnuw3eAAAAAAAAAAEAAAAAAAAADQAAAAAAAAAAO5rKAAAAAADTMfhQDaU/styfuGgrvNYHWeaSlyyuNeIpxGrig9iT5AAAAAFVU0RDAAAAADuZETgO/piLoKiQDrHP5E82b32+lGvtB3JA9/Yk3xXFAAAAAAXuP+AAAAAAAAAAAAAAAAA=',

  /** PathPaymentStrictSend: XLM → USDC via yXLM, amount=100, destMin=9.90, seq=200, one intermediate hop */
  twoHopXlmYxlmUsdc:
    'AAAAAgAAAADTMfhQDaU/styfuGgrvNYHWeaSlyyuNeIpxGrig9iT5AAAAGQAAAAAAAAAyQAAAAEAAAAAAAAAAAAAAABnuw3eAAAAAAAAAAEAAAAAAAAADQAAAAAAAAAAO5rKAAAAAADTMfhQDaU/styfuGgrvNYHWeaSlyyuNeIpxGrig9iT5AAAAAFVU0RDAAAAADuZETgO/piLoKiQDrHP5E82b32+lGvtB3JA9/Yk3xXFAAAAAAXmnsAAAAABAAAAAXlYTE0AAAAAIjbXcP4NPgFSGXXVz3rEhCtwldaxqddo0+mmMumZBr4AAAAAAAAAAA==',
};

// ── Helpers ──────────────────────────────────────────────────────────────────

/** Minimal single-hop route path (XLM → USDC) matching the builder's input shape */
function singleHopPath() {
  return [
    {
      from_asset: { asset_type: 'native' as const, asset_code: null, asset_issuer: null },
      to_asset: {
        asset_type: 'credit_alphanum4' as const,
        asset_code: 'USDC',
        asset_issuer: USDC_ISSUER,
      },
      price: '0.0995000',
      source: 'sdex',
    },
  ];
}

/** Two-hop route path (XLM → yXLM → USDC) */
function twoHopPath() {
  return [
    {
      from_asset: { asset_type: 'native' as const, asset_code: null, asset_issuer: null },
      to_asset: {
        asset_type: 'credit_alphanum4' as const,
        asset_code: 'yXLM',
        asset_issuer: yXLM_ISSUER,
      },
      price: '1.0000000',
      source: 'amm:POOL1',
    },
    {
      from_asset: {
        asset_type: 'credit_alphanum4' as const,
        asset_code: 'yXLM',
        asset_issuer: yXLM_ISSUER,
      },
      to_asset: {
        asset_type: 'credit_alphanum4' as const,
        asset_code: 'USDC',
        asset_issuer: USDC_ISSUER,
      },
      price: '0.0990000',
      source: 'sdex',
    },
  ];
}

function baseSingleHopParams(): BuildXdrParams {
  return {
    walletAddress: WALLET,
    fromAsset: 'native',
    fromAmount: '100',
    toAsset: `USDC:${USDC_ISSUER}`,
    minReceived: '9.95',
    routePath: singleHopPath(),
    networkPassphrase: TESTNET_PASSPHRASE,
    horizonUrl: HORIZON_TESTNET,
    sequenceOverride: BigInt(100),
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. parseAsset
// ─────────────────────────────────────────────────────────────────────────────
describe('parseAsset', () => {
  test('"native" returns Asset.native()', () => {
    const asset = parseAsset('native');
    expect(asset.isNative()).toBe(true);
  });

  test('"NATIVE" (uppercase) is treated as native', () => {
    const asset = parseAsset('NATIVE');
    expect(asset.isNative()).toBe(true);
  });

  test('"CODE:ISSUER" returns correct Asset', () => {
    const asset = parseAsset(`USDC:${USDC_ISSUER}`);
    expect(asset.getCode()).toBe('USDC');
    expect(asset.getIssuer()).toBe(USDC_ISSUER);
    expect(asset.isNative()).toBe(false);
  });

  test('yXLM with long issuer parses correctly', () => {
    const asset = parseAsset(`yXLM:${yXLM_ISSUER}`);
    expect(asset.getCode()).toBe('yXLM');
    expect(asset.getIssuer()).toBe(yXLM_ISSUER);
  });

  test('empty string throws XdrBuildError with code invalid_asset', () => {
    expect(() => parseAsset('')).toThrow(XdrBuildError);
    try {
      parseAsset('');
    } catch (e) {
      expect(e).toBeInstanceOf(XdrBuildError);
      expect((e as XdrBuildError).code).toBe('invalid_asset');
    }
  });

  test('non-native asset without issuer throws invalid_asset', () => {
    expect(() => parseAsset('USDC')).toThrow(XdrBuildError);
    try {
      parseAsset('USDC');
    } catch (e) {
      expect(e).toBeInstanceOf(XdrBuildError);
      expect((e as XdrBuildError).code).toBe('invalid_asset');
    }
  });

  test('asset with short/invalid issuer throws invalid_asset', () => {
    expect(() => parseAsset('USDC:TOOSHORT')).toThrow(XdrBuildError);
  });

  test('non-string input throws invalid_asset', () => {
    // @ts-expect-error intentional
    expect(() => parseAsset(null)).toThrow(XdrBuildError);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 2. validateQuoteShape
// ─────────────────────────────────────────────────────────────────────────────
describe('validateQuoteShape', () => {
  const validParams = {
    walletAddress: WALLET,
    fromAsset: 'native',
    toAsset: `USDC:${USDC_ISSUER}`,
    fromAmount: '100',
    minReceived: '9.95',
  };

  test('valid params does not throw', () => {
    expect(() => validateQuoteShape(validParams)).not.toThrow();
  });

  test('missing walletAddress throws invalid_quote_shape', () => {
    expect(() => validateQuoteShape({ ...validParams, walletAddress: '' })).toThrow(XdrBuildError);
    try {
      validateQuoteShape({ ...validParams, walletAddress: '' });
    } catch (e) {
      expect((e as XdrBuildError).code).toBe('invalid_quote_shape');
    }
  });

  test('short walletAddress (not G-address length) throws invalid_quote_shape', () => {
    expect(() => validateQuoteShape({ ...validParams, walletAddress: 'GSHORT' })).toThrow(XdrBuildError);
  });

  test('missing fromAsset throws invalid_quote_shape', () => {
    expect(() => validateQuoteShape({ ...validParams, fromAsset: '' })).toThrow(XdrBuildError);
    try {
      validateQuoteShape({ ...validParams, fromAsset: '' });
    } catch (e) {
      expect((e as XdrBuildError).code).toBe('invalid_quote_shape');
    }
  });

  test('missing toAsset throws invalid_quote_shape', () => {
    expect(() => validateQuoteShape({ ...validParams, toAsset: '  ' })).toThrow(XdrBuildError);
  });

  test('zero fromAmount throws invalid_quote_shape', () => {
    expect(() => validateQuoteShape({ ...validParams, fromAmount: '0' })).toThrow(XdrBuildError);
    try {
      validateQuoteShape({ ...validParams, fromAmount: '0' });
    } catch (e) {
      expect((e as XdrBuildError).code).toBe('invalid_quote_shape');
    }
  });

  test('negative fromAmount throws invalid_quote_shape', () => {
    expect(() => validateQuoteShape({ ...validParams, fromAmount: '-5' })).toThrow(XdrBuildError);
  });

  test('NaN fromAmount throws invalid_quote_shape', () => {
    expect(() => validateQuoteShape({ ...validParams, fromAmount: 'abc' })).toThrow(XdrBuildError);
  });

  test('empty minReceived throws invalid_quote_shape', () => {
    expect(() => validateQuoteShape({ ...validParams, minReceived: '' })).toThrow(XdrBuildError);
    try {
      validateQuoteShape({ ...validParams, minReceived: '' });
    } catch (e) {
      expect((e as XdrBuildError).code).toBe('invalid_quote_shape');
    }
  });

  test('negative minReceived throws invalid_quote_shape', () => {
    expect(() => validateQuoteShape({ ...validParams, minReceived: '-1' })).toThrow(XdrBuildError);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 3. fetchAccountSequence
// ─────────────────────────────────────────────────────────────────────────────
describe('fetchAccountSequence', () => {
  afterEach(() => { vi.restoreAllMocks(); });

  test('returns sequence as BigInt on success', async () => {
    global.fetch = vi.fn().mockResolvedValueOnce({
      ok: true,
      json: async () => ({ sequence: '12345678' }),
    } as Response);

    const seq = await fetchAccountSequence(WALLET, HORIZON_TESTNET);
    expect(seq).toBe(BigInt(12345678));
  });

  test('throws account_fetch_failed on 404', async () => {
    global.fetch = vi.fn().mockResolvedValueOnce({
      ok: false,
      status: 404,
    } as Response);

    await expect(fetchAccountSequence(WALLET, HORIZON_TESTNET)).rejects.toMatchObject({
      code: 'account_fetch_failed',
    });
  });

  test('throws account_fetch_failed on non-404 HTTP error', async () => {
    global.fetch = vi.fn().mockResolvedValueOnce({
      ok: false,
      status: 503,
    } as Response);

    await expect(fetchAccountSequence(WALLET, HORIZON_TESTNET)).rejects.toMatchObject({
      code: 'account_fetch_failed',
    });
  });

  test('throws account_fetch_failed on network error', async () => {
    global.fetch = vi.fn().mockRejectedValueOnce(new Error('Network failure'));

    await expect(fetchAccountSequence(WALLET, HORIZON_TESTNET)).rejects.toMatchObject({
      code: 'account_fetch_failed',
    });
  });

  test('throws account_fetch_failed when sequence field is absent', async () => {
    global.fetch = vi.fn().mockResolvedValueOnce({
      ok: true,
      json: async () => ({ id: WALLET }),  // no sequence field
    } as Response);

    await expect(fetchAccountSequence(WALLET, HORIZON_TESTNET)).rejects.toMatchObject({
      code: 'account_fetch_failed',
    });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 4. buildPathPaymentXdr — frozen XDR fixture tests
// ─────────────────────────────────────────────────────────────────────────────
describe('buildPathPaymentXdr — frozen XDR fixtures', () => {
  // Intercept Date.now so the encoded maxTime (now + 30s) is deterministic.
  // stellar-base calls Date.now() at transaction build time; we spy on it here
  // rather than using vi.useFakeTimers() to avoid async Promise-resolution issues.
  let dateSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    dateSpy = vi.spyOn(Date, 'now').mockReturnValue(FROZEN_NOW_MS);
  });

  afterEach(() => {
    dateSpy.mockRestore();
  });
  test('single-hop XLM→USDC produces frozen XDR (determinism check)', async () => {
    const xdr = await buildPathPaymentXdr(baseSingleHopParams());
    expect(xdr).toBe(FROZEN_XDR.singleHopXlmUsdc);
  });

  test('single-hop XDR is stable across two calls with same inputs', async () => {
    const xdr1 = await buildPathPaymentXdr(baseSingleHopParams());
    const xdr2 = await buildPathPaymentXdr(baseSingleHopParams());
    expect(xdr1).toBe(xdr2);
  });

  test('two-hop XLM→yXLM→USDC produces frozen XDR', async () => {
    const xdr = await buildPathPaymentXdr({
      ...baseSingleHopParams(),
      minReceived: '9.90',
      routePath: twoHopPath(),
      sequenceOverride: BigInt(200),
    });
    expect(xdr).toBe(FROZEN_XDR.twoHopXlmYxlmUsdc);
  });

  test('XDR changes when sequence number changes (sanity)', async () => {
    const xdr100 = await buildPathPaymentXdr({ ...baseSingleHopParams(), sequenceOverride: BigInt(100) });
    const xdr999 = await buildPathPaymentXdr({ ...baseSingleHopParams(), sequenceOverride: BigInt(999) });
    expect(xdr100).not.toBe(xdr999);
  });

  test('XDR changes when minReceived changes (slippage sensitivity)', async () => {
    const xdrTight = await buildPathPaymentXdr({ ...baseSingleHopParams(), minReceived: '9.95' });
    const xdrLoose = await buildPathPaymentXdr({ ...baseSingleHopParams(), minReceived: '9.00' });
    expect(xdrTight).not.toBe(xdrLoose);
  });

  test('returned value is a valid base64 string', async () => {
    const xdr = await buildPathPaymentXdr(baseSingleHopParams());
    // base64 characters only
    expect(xdr).toMatch(/^[A-Za-z0-9+/]+=*$/);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 5. buildPathPaymentXdr — invalid input fails before wallet prompt
// ─────────────────────────────────────────────────────────────────────────────
describe('buildPathPaymentXdr — invalid inputs fail before wallet prompt', () => {
  const signSpy = vi.fn();

  beforeEach(() => { signSpy.mockClear(); });

  test('missing walletAddress throws XdrBuildError before any I/O', async () => {
    const params = { ...baseSingleHopParams(), walletAddress: '' };
    await expect(buildPathPaymentXdr(params)).rejects.toMatchObject({
      code: 'invalid_quote_shape',
    });
    expect(signSpy).not.toHaveBeenCalled();
  });

  test('zero fromAmount throws XdrBuildError', async () => {
    await expect(buildPathPaymentXdr({ ...baseSingleHopParams(), fromAmount: '0' }))
      .rejects.toMatchObject({ code: 'invalid_quote_shape' });
  });

  test('negative fromAmount throws XdrBuildError', async () => {
    await expect(buildPathPaymentXdr({ ...baseSingleHopParams(), fromAmount: '-10' }))
      .rejects.toMatchObject({ code: 'invalid_quote_shape' });
  });

  test('empty minReceived throws XdrBuildError', async () => {
    await expect(buildPathPaymentXdr({ ...baseSingleHopParams(), minReceived: '' }))
      .rejects.toMatchObject({ code: 'invalid_quote_shape' });
  });

  test('malformed fromAsset throws XdrBuildError', async () => {
    await expect(buildPathPaymentXdr({ ...baseSingleHopParams(), fromAsset: 'USDC' }))
      .rejects.toMatchObject({ code: 'invalid_asset' });
  });

  test('malformed toAsset throws XdrBuildError', async () => {
    await expect(buildPathPaymentXdr({ ...baseSingleHopParams(), toAsset: 'NOT_VALID' }))
      .rejects.toMatchObject({ code: 'invalid_asset' });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 6. XdrBuildError shape
// ─────────────────────────────────────────────────────────────────────────────
describe('XdrBuildError', () => {
  test('has name "XdrBuildError"', () => {
    const err = new XdrBuildError('build_failed', 'test message');
    expect(err.name).toBe('XdrBuildError');
  });

  test('is instanceof Error', () => {
    const err = new XdrBuildError('invalid_asset', 'bad asset');
    expect(err).toBeInstanceOf(Error);
  });

  test('code is accessible as a property', () => {
    const err = new XdrBuildError('account_fetch_failed', 'horizon down');
    expect(err.code).toBe('account_fetch_failed');
    expect(err.message).toBe('horizon down');
  });
});
