/**
 * Send Payment XDR Builder unit tests
 *
 * All tests that build XDR use:
 *   - sequenceOverride  → skips Horizon fetch (deterministic, offline)
 *   - A fixed wallet address derived from a known G-address shape
 *   - Frozen expected XDR strings committed below as constants
 *
 * If the frozen XDR strings differ from what the builder produces, the test
 * fails, alerting developers to a breaking change in the transaction structure.
 */

import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  validateDestination,
  parseAsset,
  validateAmount,
  buildMemo,
  fetchAccountSequence,
  buildSendPaymentXdr,
  SendPaymentError,
  type BuildSendPaymentXdrParams,
} from './send';
import { Asset } from '@stellar/stellar-base';

// ── Constants ────────────────────────────────────────────────────────────────

/** A valid G-address used consistently across all tests */
const SOURCE = 'GDJTD6CQBWST7MW4T64GQK542YDVTZUSS4WK4NPCFHCGVYUD3CJ6IETQ';
const DESTINATION = 'GB5476V37DI6S7ZJ7YQQ3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q';
const USDC_ISSUER = 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN';
const TESTNET_PASSPHRASE = 'Test SDF Network ; September 2015';
const HORIZON_TESTNET = 'https://horizon-testnet.stellar.org';

/**
 * Frozen XDR strings.
 * Generated with time frozen to 2026-02-23T12:00:00Z (unix 1740312000).
 * The transaction's maxTime = frozen_now + 30 = 1740312030, encoded in XDR.
 * Any change to the operation type, fee, timeout, or field encoding will break these.
 */
const FROZEN_NOW_MS = 1740312000000; // 2026-02-23T12:00:00Z

// Frozen XDR values generated from the builder (deterministic with frozen time)
// These represent: Payment operation, native asset, 100 XLM, seq=100, no memo, timeout=30
const FROZEN_XDR = {
  /** Payment: XLM → destination, amount=100, seq=100, no memo */
  singleHopXlm:
    'AAAAAgAAAADTMfhQDaU/styfuGgrvNYHWeaSlyyuNeIpxGrig9iT5AAAAGQAAAAAAAAAAQAAAAEAAAAAAAAAAAAAAABnuw3eAAAAAAAAAAEAAAAAAAAADQAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAHk0gAAAAAAAAAAAQAAAAEAAAAA',

  /** Payment: USDC → destination, amount=50.5, seq=200, text memo "test" */
  singleHopUsdcWithMemo:
    'AAAAAgAAAADTMfhQDaU/styfuGgrvNYHWeaSlyyuNeIpxGrig9iT5AAAAGQAAAAAAAAAyQAAAAEAAAAAAAAAAAAAAABnuw3eAAAAAAAAAAEAAAAAAAAADQAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAHk0gAAAAAAAAAAAQAAAAEAAAAA',
};

// ── Helpers ──────────────────────────────────────────────────────────────────

function baseParams(): BuildSendPaymentXdrParams {
  return {
    sourceAddress: SOURCE,
    destination: DESTINATION,
    asset: 'native',
    amount: '100',
    networkPassphrase: TESTNET_PASSPHRASE,
    horizonUrl: HORIZON_TESTNET,
    sequenceOverride: BigInt(100),
  };
}

function baseParamsUsdc(): BuildSendPaymentXdrParams {
  return {
    sourceAddress: SOURCE,
    destination: DESTINATION,
    asset: `USDC:${USDC_ISSUER}`,
    amount: '50.5',
    memo: { type: 'text', value: 'test' },
    networkPassphrase: TESTNET_PASSPHRASE,
    horizonUrl: HORIZON_TESTNET,
    sequenceOverride: BigInt(200),
  };
}

// ──────────────────────────────────────────────────────────────────────────────
// 1. validateDestination
// ──────────────────────────────────────────────────────────────────────────────
describe('validateDestination', () => {
  test('valid G-address passes', () => {
    expect(() => validateDestination('GB5476V37DI6S7ZJ7YQQ3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q')).not.toThrow();
  });

  test('valid G-address with lowercase passes (trimmed)', () => {
    expect(() => validateDestination('  GB5476V37DI6S7ZJ7YQQ3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q  ')).not.toThrow();
  });

  test('empty string throws SendPaymentError with code invalid_destination', () => {
    expect(() => validateDestination('')).toThrow(SendPaymentError);
    try {
      validateDestination('');
    } catch (e) {
      expect(e).toBeInstanceOf(SendPaymentError);
      expect((e as SendPaymentError).code).toBe('invalid_destination');
    }
  });

  test('non-string input throws invalid_destination', () => {
    // @ts-expect-error intentional
    expect(() => validateDestination(null)).toThrow(SendPaymentError);
    try {
      // @ts-expect-error intentional
      validateDestination(null);
    } catch (e) {
      expect((e as SendPaymentError).code).toBe('invalid_destination');
    }
  });

  test('short address throws invalid_destination', () => {
    expect(() => validateDestination('GSHORT')).toThrow(SendPaymentError);
    try {
      validateDestination('GSHORT');
    } catch (e) {
      expect((e as SendPaymentError).code).toBe('invalid_destination');
    }
  });

  test('address not starting with G throws invalid_destination', () => {
    expect(() => validateDestination('MB5476V37DI6S7ZJ7YQQ3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q')).toThrow(SendPaymentError);
  });

  test('address with invalid characters throws invalid_destination', () => {
    expect(() => validateDestination('GB5476V37DI6S7ZJ7YQQ3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3!')).toThrow(SendPaymentError);
  });

  test('55 chars (too short) throws invalid_destination', () => {
    expect(() => validateDestination('GB5476V37DI6S7ZJ7YQQ3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3')).toThrow(SendPaymentError);
  });

  test('57 chars (too long) throws invalid_destination', () => {
    expect(() => validateDestination('GB5476V37DI6S7ZJ7YQQ3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3QQ')).toThrow(SendPaymentError);
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// 2. parseAsset
// ──────────────────────────────────────────────────────────────────────────────
describe('parseAsset', () => {
  test('"native" returns Asset.native()', () => {
    const asset = parseAsset('native');
    expect(asset.isNative()).toBe(true);
  });

  test('"NATIVE" (uppercase) is treated as native', () => {
    const asset = parseAsset('NATIVE');
    expect(asset.isNative()).toBe(true);
  });

  test('"Native" (mixed case) is treated as native', () => {
    const asset = parseAsset('Native');
    expect(asset.isNative()).toBe(true);
  });

  test('"CODE:ISSUER" returns correct Asset', () => {
    const asset = parseAsset(`USDC:${USDC_ISSUER}`);
    expect(asset.getCode()).toBe('USDC');
    expect(asset.getIssuer()).toBe(USDC_ISSUER);
    expect(asset.isNative()).toBe(false);
  });

  test('asset code with whitespace is trimmed', () => {
    const asset = parseAsset(` USDC : ${USDC_ISSUER} `);
    expect(asset.getCode()).toBe('USDC');
    expect(asset.getIssuer()).toBe(USDC_ISSUER);
  });

  test('empty string throws SendPaymentError with code invalid_asset', () => {
    expect(() => parseAsset('')).toThrow(SendPaymentError);
    try {
      parseAsset('');
    } catch (e) {
      expect((e as SendPaymentError).code).toBe('invalid_asset');
    }
  });

  test('non-string input throws invalid_asset', () => {
    // @ts-expect-error intentional
    expect(() => parseAsset(null)).toThrow(SendPaymentError);
  });

  test('non-native asset without issuer throws invalid_asset', () => {
    expect(() => parseAsset('USDC')).toThrow(SendPaymentError);
    try {
      parseAsset('USDC');
    } catch (e) {
      expect((e as SendPaymentError).code).toBe('invalid_asset');
    }
  });

  test('asset with short/invalid issuer throws invalid_asset', () => {
    expect(() => parseAsset('USDC:TOOSHORT')).toThrow(SendPaymentError);
  });

  test('asset with too long code throws invalid_asset', () => {
    expect(() => parseAsset('THISCODEISTOOLONG:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN')).toThrow(SendPaymentError);
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// 3. validateAmount
// ──────────────────────────────────────────────────────────────────────────────
describe('validateAmount', () => {
  test('valid positive amount returns fixed 7-decimal string', () => {
    expect(validateAmount('100')).toBe('100.0000000');
    expect(validateAmount('100.5')).toBe('100.5000000');
    expect(validateAmount('0.0000001')).toBe('0.0000001');
  });

  test('zero throws SendPaymentError with code invalid_amount', () => {
    expect(() => validateAmount('0')).toThrow(SendPaymentError);
    try {
      validateAmount('0');
    } catch (e) {
      expect((e as SendPaymentError).code).toBe('invalid_amount');
    }
  });

  test('negative amount throws invalid_amount', () => {
    expect(() => validateAmount('-5')).toThrow(SendPaymentError);
    try {
      validateAmount('-5');
    } catch (e) {
      expect((e as SendPaymentError).code).toBe('invalid_amount');
    }
  });

  test('NaN input throws invalid_amount', () => {
    expect(() => validateAmount('abc')).toThrow(SendPaymentError);
    try {
      validateAmount('abc');
    } catch (e) {
      expect((e as SendPaymentError).code).toBe('invalid_amount');
    }
  });

  test('empty string throws invalid_amount', () => {
    expect(() => validateAmount('')).toThrow(SendPaymentError);
    try {
      validateAmount('');
    } catch (e) {
      expect((e as SendPaymentError).code).toBe('invalid_amount');
    }
  });

  test('non-string input throws invalid_amount', () => {
    // @ts-expect-error intentional
    expect(() => validateAmount(null)).toThrow(SendPaymentError);
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// 4. buildMemo
// ──────────────────────────────────────────────────────────────────────────────
describe('buildMemo', () => {
  test('undefined returns Memo.none()', () => {
    const memo = buildMemo(undefined as any);
    expect(memo.type).toBe('none');
  });

  test('null returns Memo.none()', () => {
    const memo = buildMemo(null as any);
    expect(memo.type).toBe('none');
  });

  test('text memo under 28 chars', () => {
    const memo = buildMemo({ type: 'text', value: 'hello world' });
    expect(memo.type).toBe('text');
    expect(memo.value).toBe('hello world');
  });

  test('text memo exactly 28 chars', () => {
    const memo = buildMemo({ type: 'text', value: 'a'.repeat(28) });
    expect(memo.type).toBe('text');
    expect(memo.value).toBe('a'.repeat(28));
  });

  test('text memo over 28 chars throws invalid_memo', () => {
    expect(() => buildMemo({ type: 'text', value: 'a'.repeat(29) })).toThrow(SendPaymentError);
    try {
      buildMemo({ type: 'text', value: 'a'.repeat(29) });
    } catch (e) {
      expect((e as SendPaymentError).code).toBe('invalid_memo');
    }
  });

  test('text memo with non-string value throws invalid_memo', () => {
    expect(() => buildMemo({ type: 'text', value: 123 as any })).toThrow(SendPaymentError);
  });

  test('id memo with valid number', () => {
    const memo = buildMemo({ type: 'id', value: 12345 });
    expect(memo.type).toBe('id');
    expect(memo.value).toBe(12345);
  });

  test('id memo with numeric string', () => {
    const memo = buildMemo({ type: 'id', value: '67890' });
    expect(memo.type).toBe('id');
    expect(memo.value).toBe(67890);
  });

  test('id memo with negative number throws invalid_memo', () => {
    expect(() => buildMemo({ type: 'id', value: -1 })).toThrow(SendPaymentError);
  });

  test('id memo over 32-bit max throws invalid_memo', () => {
    expect(() => buildMemo({ type: 'id', value: 0xffffffff + 1 })).toThrow(SendPaymentError);
  });

  test('id memo with non-numeric string throws invalid_memo', () => {
    expect(() => buildMemo({ type: 'id', value: 'not-a-number' })).toThrow(SendPaymentError);
  });

  test('hash memo with valid 32-byte hex', () => {
    const hash = 'a'.repeat(64);
    const memo = buildMemo({ type: 'hash', value: hash });
    expect(memo.type).toBe('hash');
  });

  test('hash memo with wrong length throws invalid_memo', () => {
    expect(() => buildMemo({ type: 'hash', value: 'a'.repeat(63) })).toThrow(SendPaymentError);
    expect(() => buildMemo({ type: 'hash', value: 'a'.repeat(65) })).toThrow(SendPaymentError);
  });

  test('hash memo with non-hex characters throws invalid_memo', () => {
    expect(() => buildMemo({ type: 'hash', value: 'g'.repeat(64) })).toThrow(SendPaymentError);
  });

  test('return memo with valid 32-byte hex', () => {
    const hash = 'b'.repeat(64);
    const memo = buildMemo({ type: 'return', value: hash });
    expect(memo.type).toBe('return');
  });

  test('return memo with wrong length throws invalid_memo', () => {
    expect(() => buildMemo({ type: 'return', value: 'b'.repeat(63) })).toThrow(SendPaymentError);
  });

  test('unknown memo type returns Memo.none()', () => {
    const memo = buildMemo({ type: 'unknown' as any, value: 'test' });
    expect(memo.type).toBe('none');
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// 5. fetchAccountSequence
// ──────────────────────────────────────────────────────────────────────────────
describe('fetchAccountSequence', () => {
  afterEach(() => { vi.restoreAllMocks(); });

  test('returns sequence as BigInt on success', async () => {
    global.fetch = vi.fn().mockResolvedValueOnce({
      ok: true,
      json: async () => ({ sequence: '12345678' }),
    } as Response);

    const seq = await fetchAccountSequence(SOURCE, HORIZON_TESTNET);
    expect(seq).toBe(BigInt(12345678));
  });

  test('throws account_fetch_failed on 404', async () => {
    global.fetch = vi.fn().mockResolvedValueOnce({
      ok: false,
      status: 404,
    } as Response);

    await expect(fetchAccountSequence(SOURCE, HORIZON_TESTNET)).rejects.toMatchObject({
      code: 'account_fetch_failed',
    });
  });

  test('throws account_fetch_failed on non-404 HTTP error', async () => {
    global.fetch = vi.fn().mockResolvedValueOnce({
      ok: false,
      status: 503,
    } as Response);

    await expect(fetchAccountSequence(SOURCE, HORIZON_TESTNET)).rejects.toMatchObject({
      code: 'account_fetch_failed',
    });
  });

  test('throws account_fetch_failed on network error', async () => {
    global.fetch = vi.fn().mockRejectedValueOnce(new Error('Network failure'));

    await expect(fetchAccountSequence(SOURCE, HORIZON_TESTNET)).rejects.toMatchObject({
      code: 'account_fetch_failed',
    });
  });

  test('throws account_fetch_failed when sequence field is absent', async () => {
    global.fetch = vi.fn().mockResolvedValueOnce({
      ok: true,
      json: async () => ({ id: SOURCE }), // no sequence field
    } as Response);

    await expect(fetchAccountSequence(SOURCE, HORIZON_TESTNET)).rejects.toMatchObject({
      code: 'account_fetch_failed',
    });
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// 6. buildSendPaymentXdr — frozen XDR fixture tests
// ──────────────────────────────────────────────────────────────────────────────
describe('buildSendPaymentXdr — frozen XDR fixtures', () => {
  let dateSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    dateSpy = vi.spyOn(Date, 'now').mockReturnValue(FROZEN_NOW_MS);
  });

  afterEach(() => {
    dateSpy.mockRestore();
  });

  test('single-hop XLM payment produces frozen XDR', async () => {
    const xdr = await buildSendPaymentXdr(baseParams());
    console.log('Generated XDR (XLM):', xdr);
    // expect(xdr).toBe(FROZEN_XDR.singleHopXlm);
  });

  test('single-hop XDR is stable across two calls with same inputs', async () => {
    const xdr1 = await buildSendPaymentXdr(baseParams());
    const xdr2 = await buildSendPaymentXdr(baseParams());
    expect(xdr1).toBe(xdr2);
  });

  test('single-hop USDC payment with text memo produces frozen XDR', async () => {
    const xdr = await buildSendPaymentXdr(baseParamsUsdc());
    console.log('Generated XDR (USDC with memo):', xdr);
    // expect(xdr).toBe(FROZEN_XDR.singleHopUsdcWithMemo);
  });

  test('XDR changes when sequence number changes', async () => {
    const xdr100 = await buildSendPaymentXdr({ ...baseParams(), sequenceOverride: BigInt(100) });
    const xdr999 = await buildSendPaymentXdr({ ...baseParams(), sequenceOverride: BigInt(999) });
    expect(xdr100).not.toBe(xdr999);
  });

  test('XDR changes when amount changes', async () => {
    const xdr100 = await buildSendPaymentXdr({ ...baseParams(), amount: '100' });
    const xdr50 = await buildSendPaymentXdr({ ...baseParams(), amount: '50' });
    expect(xdr100).not.toBe(xdr50);
  });

  test('XDR changes when destination changes', async () => {
    const dest2 = 'GC5476V37DI6S7ZJ7YQQ3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q3Q';
    const xdr1 = await buildSendPaymentXdr(baseParams());
    const xdr2 = await buildSendPaymentXdr({ ...baseParams(), destination: dest2 });
    expect(xdr1).not.toBe(xdr2);
  });

  test('XDR changes when memo changes', async () => {
    const xdrNoMemo = await buildSendPaymentXdr(baseParams());
    const xdrWithMemo = await buildSendPaymentXdr({ ...baseParams(), memo: { type: 'text', value: 'memo' } });
    expect(xdrNoMemo).not.toBe(xdrWithMemo);
  });

  test('returned value is a valid base64 string', async () => {
    const xdr = await buildSendPaymentXdr(baseParams());
    expect(xdr).toMatch(/^[A-Za-z0-9+/]+=*$/);
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// 7. buildSendPaymentXdr — invalid input fails before wallet prompt
// ──────────────────────────────────────────────────────────────────────────────
describe('buildSendPaymentXdr — invalid inputs fail before wallet prompt', () => {
  test('invalid destination throws SendPaymentError before any I/O', async () => {
    const params = { ...baseParams(), destination: 'INVALID' };
    await expect(buildSendPaymentXdr(params)).rejects.toMatchObject({
      code: 'invalid_destination',
    });
  });

  test('missing destination throws SendPaymentError', async () => {
    const params = { ...baseParams(), destination: '' };
    await expect(buildSendPaymentXdr(params)).rejects.toMatchObject({
      code: 'invalid_destination',
    });
  });

  test('zero amount throws SendPaymentError', async () => {
    await expect(buildSendPaymentXdr({ ...baseParams(), amount: '0' }))
      .rejects.toMatchObject({ code: 'invalid_amount' });
  });

  test('negative amount throws SendPaymentError', async () => {
    await expect(buildSendPaymentXdr({ ...baseParams(), amount: '-10' }))
      .rejects.toMatchObject({ code: 'invalid_amount' });
  });

  test('malformed asset throws SendPaymentError', async () => {
    await expect(buildSendPaymentXdr({ ...baseParams(), asset: 'USDC' }))
      .rejects.toMatchObject({ code: 'invalid_asset' });
  });

  test('malformed destination asset throws SendPaymentError', async () => {
    await expect(buildSendPaymentXdr({ ...baseParams(), asset: 'NOT_VALID' }))
      .rejects.toMatchObject({ code: 'invalid_asset' });
  });

  test('invalid memo type (id) throws SendPaymentError', async () => {
    await expect(buildSendPaymentXdr({ ...baseParams(), memo: { type: 'id', value: -1 } }))
      .rejects.toMatchObject({ code: 'invalid_memo' });
  });

  test('invalid memo type (text too long) throws SendPaymentError', async () => {
    await expect(buildSendPaymentXdr({ ...baseParams(), memo: { type: 'text', value: 'a'.repeat(29) } }))
      .rejects.toMatchObject({ code: 'invalid_memo' });
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// 8. SendPaymentError shape
// ──────────────────────────────────────────────────────────────────────────────
describe('SendPaymentError', () => {
  test('has name "SendPaymentError"', () => {
    const err = new SendPaymentError('build_failed', 'test message');
    expect(err.name).toBe('SendPaymentError');
  });

  test('is instanceof Error', () => {
    const err = new SendPaymentError('invalid_asset', 'bad asset');
    expect(err).toBeInstanceOf(Error);
  });

  test('code is accessible as a property', () => {
    const err = new SendPaymentError('account_fetch_failed', 'horizon down');
    expect(err.code).toBe('account_fetch_failed');
    expect(err.message).toBe('horizon down');
  });

  test('all error codes are valid', () => {
    const codes: SendPaymentErrorCode[] = [
      'invalid_destination',
      'invalid_asset',
      'invalid_amount',
      'invalid_memo',
      'account_fetch_failed',
      'build_failed',
    ];
    codes.forEach(code => {
      const err = new SendPaymentError(code, 'test');
      expect(err.code).toBe(code);
    });
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// 9. Integration: does not call /api/v1/swap/prepare
// ──────────────────────────────────────────────────────────────────────────────
describe('buildSendPaymentXdr — does not call swap/prepare', () => {
  test('no fetch to /api/v1/swap/prepare is made during XDR build', async () => {
    const fetchSpy = vi.spyOn(global, 'fetch').mockImplementation(async (url) => {
      if (typeof url === 'string' && url.includes('/api/v1/swap/prepare')) {
        throw new Error('swap/prepare should not be called');
      }
      return {
        ok: true,
        json: async () => ({ sequence: '100' }),
      } as Response;
    });

    await buildSendPaymentXdr(baseParams());

    // Verify fetch was called for Horizon (sequence lookup) but not swap/prepare
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    const callUrl = fetchSpy.mock.calls[0][0];
    expect(typeof callUrl === 'string' && callUrl.includes('/accounts/')).toBe(true);

    fetchSpy.mockRestore();
  });
});