import { describe, expect, it, vi, afterEach } from 'vitest';
import { StellarRouteClient, StellarRouteApiError } from './client.js';

const NATIVE = { asset_type: 'native' } as import('./types.js').Asset;
const USDC = { asset_type: 'credit_alphanum4', asset_code: 'USDC', asset_issuer: 'GDUKMGUGDZQK6YH...' } as import('./types.js').Asset;

function ok(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

function apiError(code: string, message: string, status: number): Response {
  return new Response(JSON.stringify({ error: code, message }), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe('Insufficient Liquidity Scenarios', () => {
  it('returns an error when no route satisfies the trade size', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiError('not_found', 'Insufficient liquidity for this trade size', 404)
    );

    const client = new StellarRouteClient({ retries: 0 });
    const hugeTradeSize = 10_000_000_000;

    let error: StellarRouteApiError | undefined;
    try {
      await client.getQuote('native', 'USDC:GDUKMGUGDZQK6YH...', hugeTradeSize);
    } catch (e: any) {
      error = e;
    }

    expect(error).toBeDefined();
    expect(error?.status).toBe(404);
    expect(error?.code).toBe('not_found');
    expect(error?.message).toMatch(/Insufficient liquidity/i);
    expect(error?.isNetworkError()).toBe(false);
  });

  it('rejects trade when available liquidity is below minimum operational thresholds', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiError('validation_error', 'Liquidity below minimum thresholds', 400)
    );

    const client = new StellarRouteClient({ retries: 0 });
    
    let error: StellarRouteApiError | undefined;
    try {
      await client.getQuote('native', 'USDC:GDUKMGUGDZQK6YH...', 1);
    } catch (e: any) {
      error = e;
    }

    expect(error).toBeDefined();
    expect(error?.status).toBe(400);
    expect(error?.code).toBe('validation_error');
    expect(error?.message).toMatch(/Liquidity below minimum thresholds/i);
  });

  it('returns partial fill quote when only some venues have depth', async () => {
    const partialFillFixture = {
      base_asset: NATIVE,
      quote_asset: USDC,
      amount: '2000',
      price: '0.90',
      total: '1800',
      quote_type: 'sell',
      path: [
        { from_asset: NATIVE, to_asset: USDC, price: '0.92', source: 'sdex:offer1' },
        { from_asset: NATIVE, to_asset: USDC, price: '0.88', source: 'amm:poolThin' },
      ],
      timestamp: 1_700_000_004,
    };

    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(partialFillFixture));

    const client = new StellarRouteClient();
    const result = await client.getQuote('native', 'USDC:GDUKMGUGDZQK6YH...', 2000);

    expect(spy).toHaveBeenCalledTimes(1);
    expect(result.path).toHaveLength(2);
    expect(result.total).toBe('1800');
  });

  it('returns 422 stale_market_data when liquidity data is too old', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiError('stale_market_data', 'Orderbook data is older than 60 seconds', 422)
    );

    const client = new StellarRouteClient({ retries: 0 });

    let error: StellarRouteApiError | undefined;
    try {
      await client.getQuote('native', 'USDC:GDUKMGUGDZQK6YH...', 500);
    } catch (e: any) {
      error = e;
    }

    expect(error).toBeDefined();
    expect(error?.status).toBe(422);
    expect(error?.code).toBe('stale_market_data');
    expect(error?.isStaleMarketData()).toBe(true);
  });

  it('returns overloaded error when all venues are at capacity', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiError('overloaded', 'All venues at capacity', 503)
    );

    const client = new StellarRouteClient({ retries: 0 });

    let error: StellarRouteApiError | undefined;
    try {
      await client.getQuote('native', 'USDC:GDUKMGUGDZQK6YH...', 10000);
    } catch (e: any) {
      error = e;
    }

    expect(error).toBeDefined();
    expect(error?.status).toBe(503);
    expect(error?.code).toBe('overloaded');
    expect(error?.isOverloaded()).toBe(true);
  });

  it('fetch mock is verified â€” no real network calls made', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      ok({
        base_asset: NATIVE,
        quote_asset: USDC,
        amount: '100',
        price: '0.95',
        total: '95',
        quote_type: 'sell',
        path: [{ from_asset: NATIVE, to_asset: USDC, price: '0.95', source: 'sdex' }],
        timestamp: 1_700_000_005,
      })
    );

    const client = new StellarRouteClient();
    await client.getQuote('native', 'USDC:GDUKMGUGDZQK6YH...', 100);

    expect(spy).toHaveBeenCalledTimes(1);
    const calledUrl = spy.mock.calls[0]?.[0] as string;
    expect(calledUrl).toContain('localhost:8080');
    expect(calledUrl).toContain('/api/v1/quote/');
  });
});