/**
 * Cash-out (Offramp) Quote Builder unit tests
 *
 * Uses the existing offramp quote fixtures (INDICATIVE_USDC_NGN,
 * OFFRAMP_FEE_BPS) from frontend/lib/offramp/quote.ts so that
 * preview amounts stay in sync with the live quote helper.
 *
 * This module is additive-only: it does not change any existing
 * swap, quote, or OpenAPI contract behavior.
 */

import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import {
  buildCashOutQuote,
  resolveOfframpAsset,
  OfframpToolError,
  type BuildCashOutQuoteParams,
} from './offramp';

// ── Helpers ──────────────────────────────────────────────────────────

function baseParams(): BuildCashOutQuoteParams {
  return {
    asset: 'stellar-usdc',
    amount: '100',
  };
}

// ──────────────────────────────────────────────────────────────────────
// 1. resolveOfframpAsset
// ──────────────────────────────────────────────────────────────────────
describe('resolveOfframpAsset', () => {
  it('resolves by source ID (stellar-usdc)', () => {
    const asset = resolveOfframpAsset('stellar-usdc');
    expect(asset.symbol).toBe('USDC');
    expect(asset.isStellarUsdc).toBe(true);
  });

  it('resolves by source ID (stellar-xlm)', () => {
    const asset = resolveOfframpAsset('stellar-xlm');
    expect(asset.symbol).toBe('XLM');
    expect(asset.status).toBe('swap_then_offramp');
  });

  it('resolves by symbol (USDC)', () => {
    const asset = resolveOfframpAsset('USDC');
    expect(asset.symbol).toBe('USDC');
  });

  it('resolves by symbol (XLM)', () => {
    const asset = resolveOfframpAsset('XLM');
    expect(asset.symbol).toBe('XLM');
  });

  it('resolves eth-usdc by source ID', () => {
    const asset = resolveOfframpAsset('eth-usdc');
    expect(asset.symbol).toBe('USDC');
    expect(asset.status).toBe('bridge_required');
  });

  it('throws invalid_asset for empty string', () => {
    expect(() => resolveOfframpAsset('')).toThrow(OfframpToolError);
    try {
      resolveOfframpAsset('');
    } catch (e) {
      expect(e).toBeInstanceOf(OfframpToolError);
      expect((e as OfframpToolError).code).toBe('invalid_asset');
    }
  });

  it('throws invalid_asset for unknown asset', () => {
    expect(() => resolveOfframpAsset('unknown-asset')).toThrow(OfframpToolError);
    try {
      resolveOfframpAsset('unknown-asset');
    } catch (e) {
      expect(e).toBeInstanceOf(OfframpToolError);
      expect((e as OfframpToolError).code).toBe('invalid_asset');
    }
  });
});

// ──────────────────────────────────────────────────────────────────────
// 2. buildCashOutQuote — fixture NGN amounts and indicative flag
// ──────────────────────────────────────────────────────────────────────
describe('buildCashOutQuote — fixture NGN amounts and indicative', () => {
  it('returns a preview with NGN amount and indicative flag for USDC', async () => {
    const quote = await buildCashOutQuote(baseParams());

    expect(quote).not.toBeNull();
    expect(quote!.indicative).toBe(true);
    expect(quote!.sourceSymbol).toBe('USDC');
    expect(quote!.sourceAmount).toBe('100.0000000');
    // 100 USDC * 1580 NGN/USDC = 158,000 NGN (before fee)
    // fee = 0.5% of 100 = 0.5 USDC
    // net = 99.5 USDC
    // receiveNgn = 99.5 * 1580 = 157,210
    expect(quote!.receiveNgn).toBe('157,210.00');
    expect(quote!.feeUsdc).toBe('0.50');
    expect(quote!.netUsdc).toBe('99.50');
  });

  it('returns a preview for XLM with swap_then_offramp status', async () => {
    const quote = await buildCashOutQuote({
      asset: 'stellar-xlm',
      amount: '100',
    });

    expect(quote).not.toBeNull();
    expect(quote!.indicative).toBe(true);
    expect(quote!.sourceSymbol).toBe('XLM');
    // XLM → USDC at ~0.12 rate: 100 * 0.12 = 12 USDC
    // fee = 0.5% of 12 = 0.06 USDC
    // net = 11.94 USDC
    // receiveNgn = 11.94 * 1580 = 18,865.20
    expect(quote!.receiveNgn).toBe('18,865.20');
  });

  it('returns a preview for bridge-required USDC (eth-usdc)', async () => {
    const quote = await buildCashOutQuote({
      asset: 'eth-usdc',
      amount: '100',
      mode: 'bridge',
    });

    expect(quote).not.toBeNull();
    expect(quote!.indicative).toBe(true);
    expect(quote!.sourceSymbol).toBe('USDC');
    expect(quote!.receiveNgn).toBe('157,210.00');
  });

  it('returns null for empty amount', async () => {
    const quote = await buildCashOutQuote({
      ...baseParams(),
      amount: '',
    });
    expect(quote).toBeNull();
  });

  it('returns null for negative amount', async () => {
    const quote = await buildCashOutQuote({
      ...baseParams(),
      amount: '-1',
    });
    expect(quote).toBeNull();
  });
});

// ──────────────────────────────────────────────────────────────────────
// 3. Non-USDC sources keep existing status as copy
// ──────────────────────────────────────────────────────────────────────
describe('buildCashOutQuote — non-USDC status copy', () => {
  it('XLM source has swap_then_offramp status (swap required copy)', async () => {
    const asset = resolveOfframpAsset('stellar-xlm');
    expect(asset.status).toBe('swap_then_offramp');

    const quote = await buildCashOutQuote({
      asset: 'stellar-xlm',
      amount: '100',
    });

    expect(quote).not.toBeNull();
    // The quote itself is built from the existing helper;
    // the status is a property of the source asset, not the quote.
    // The tool preserves the asset's existing status for downstream
    // copy rendering (bridge or swap required).
    expect(asset.status).toBe('swap_then_offramp');
  });

  it('eth-usdc source has bridge_required status (bridge required copy)', async () => {
    const asset = resolveOfframpAsset('eth-usdc');
    expect(asset.status).toBe('bridge_required');

    const quote = await buildCashOutQuote({
      asset: 'eth-usdc',
      amount: '100',
      mode: 'bridge',
    });

    expect(quote).not.toBeNull();
    expect(asset.status).toBe('bridge_required');
  });
});

// ──────────────────────────────────────────────────────────────────────
// 4. No Paycrest secret is read in the client
// ──────────────────────────────────────────────────────────────────────
describe('buildCashOutQuote — no Paycrest secret read', () => {
  it('does not call any Paycrest endpoint', async () => {
    const fetchSpy = vi.spyOn(global, 'fetch').mockImplementation(async (url) => {
      if (typeof url === 'string' && url.includes('paycrest')) {
        throw new Error('Paycrest should not be called');
      }
      return {
        ok: true,
        json: async () => ({ sequence: '100' }),
      } as Response;
    });

    const quote = await buildCashOutQuote(baseParams());
    expect(quote).not.toBeNull();

    // Verify no Paycrest-related fetch calls were made
    const paycrestCalls = fetchSpy.mock.calls.filter(
      (call) => typeof call[0] === 'string' && call[0].includes('paycrest'),
    );
    expect(paycrestCalls).toHaveLength(0);

    fetchSpy.mockRestore();
  });
});

// ──────────────────────────────────────────────────────────────────────
// 5. Invalid input handling
// ──────────────────────────────────────────────────────────────────────
describe('buildCashOutQuote — invalid input returns null', () => {
  it('returns null for empty amount', async () => {
    const quote = await buildCashOutQuote({
      ...baseParams(),
      amount: '',
    });
    expect(quote).toBeNull();
  });

  it('returns null for zero amount', async () => {
    const quote = await buildCashOutQuote({
      ...baseParams(),
      amount: '0',
    });
    expect(quote).toBeNull();
  });

  it('returns null for negative amount', async () => {
    const quote = await buildCashOutQuote({
      ...baseParams(),
      amount: '-1',
    });
    expect(quote).toBeNull();
  });

  it('returns null for non-numeric amount', async () => {
    const quote = await buildCashOutQuote({
      ...baseParams(),
      amount: 'abc',
    });
    expect(quote).toBeNull();
  });

  it('throws OfframpToolError with invalid_asset for unknown asset', async () => {
    await expect(
      buildCashOutQuote({ asset: 'unknown', amount: '100' }),
    ).rejects.toMatchObject({
      code: 'invalid_asset',
    });
  });

  it('throws OfframpToolError with invalid_asset for empty asset', async () => {
    await expect(
      buildCashOutQuote({ asset: '', amount: '100' }),
    ).rejects.toMatchObject({
      code: 'invalid_asset',
    });
  });
});

// ──────────────────────────────────────────────────────────────────────
// 6. Mode resolution
// ──────────────────────────────────────────────────────────────────────
describe('buildCashOutQuote — mode resolution', () => {
  it('defaults to direct mode for stellar-usdc', async () => {
    const quote = await buildCashOutQuote(baseParams());
    expect(quote).not.toBeNull();
    expect(quote!.mode).toBe('direct');
  });

  it('defaults to bridge mode for eth-usdc', async () => {
    const quote = await buildCashOutQuote({
      asset: 'eth-usdc',
      amount: '100',
    });
    expect(quote).not.toBeNull();
    expect(quote!.mode).toBe('bridge');
  });

  it('uses explicit mode override when provided', async () => {
    const quote = await buildCashOutQuote({
      asset: 'stellar-usdc',
      amount: '100',
      mode: 'bridge',
    });
    expect(quote).not.toBeNull();
    expect(quote!.mode).toBe('bridge');
  });
});
