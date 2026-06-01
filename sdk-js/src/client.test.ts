import { describe, expect, it, vi, afterEach, beforeEach } from 'vitest';
import { StellarRouteClient, StellarRouteApiError, isStellarRouteApiError } from './client.js';
import type { HealthStatus, Orderbook, PairsResponse, PriceQuote } from './types.js';

// ── Fixtures ──────────────────────────────────────────────────────────────────

const NATIVE: import('./types.js').Asset = { asset_type: 'native' };
const USDC: import('./types.js').Asset = {
  asset_type: 'credit_alphanum4',
  asset_code: 'USDC',
  asset_issuer: 'GDUKMGUGDZQK6YH...',
};

const sampleHealth: HealthStatus = {
  status: 'healthy',
  version: '0.1.0',
  timestamp: '2026-03-25T12:00:00Z',
  components: { database: 'healthy', redis: 'healthy' },
};

const samplePairs: PairsResponse = {
  pairs: [
    {
      base: 'XLM',
      counter: 'USDC',
      base_asset: 'native',
      counter_asset: 'USDC:GDUKMGUGDZQK6YH...',
      offer_count: 42,
      last_updated: '2026-03-25T11:59:00Z',
    },
  ],
  total: 1,
};

const sampleOrderbook: Orderbook = {
  base_asset: NATIVE,
  quote_asset: USDC,
  bids: [{ price: '0.1050000', amount: '500.0000000', total: '52.5000000' }],
  asks: [{ price: '0.1060000', amount: '300.0000000', total: '31.8000000' }],
  timestamp: 1_740_312_000,
};

const sampleQuote: PriceQuote = {
  base_asset: NATIVE,
  quote_asset: USDC,
  amount: '100',
  price: '0.99',
  total: '99',
  quote_type: 'sell',
  path: [{ from_asset: NATIVE, to_asset: USDC, price: '0.99', source: 'sdex' }],
  timestamp: 1_717_171_717,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

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

afterEach(() => vi.restoreAllMocks());

// ── getHealth ─────────────────────────────────────────────────────────────────

describe('getHealth', () => {
  it('returns typed HealthStatus on 200', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(sampleHealth));
    const client = new StellarRouteClient();
    const result = await client.getHealth();
    expect(result.status).toBe('healthy');
    expect(result.version).toBe('0.1.0');
    expect(result.components.database).toBe('healthy');
  });

  it('calls the correct endpoint', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(sampleHealth));
    await new StellarRouteClient({ baseUrl: 'https://api.example.com' }).getHealth();
    expect(spy.mock.calls[0]?.[0]).toBe('https://api.example.com/health');
  });
});

// ── getPairs ──────────────────────────────────────────────────────────────────

describe('getPairs', () => {
  it('returns typed PairsResponse on 200', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(samplePairs));
    const result = await new StellarRouteClient().getPairs();
    expect(result.total).toBe(1);
    expect(result.pairs[0]?.base).toBe('XLM');
    expect(result.pairs[0]?.offer_count).toBe(42);
  });
});

// ── getOrderbook ──────────────────────────────────────────────────────────────

describe('getOrderbook', () => {
  it('returns typed Orderbook on 200', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(sampleOrderbook));
    const result = await new StellarRouteClient().getOrderbook('native', 'USDC');
    expect(result.bids).toHaveLength(1);
    expect(result.asks[0]?.price).toBe('0.1060000');
    expect(result.base_asset.asset_type).toBe('native');
  });

  it('URL-encodes asset identifiers with colons', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(sampleOrderbook));
    await new StellarRouteClient().getOrderbook('native', 'USDC:GDUKMGUGDZQK6YH...');
    expect(spy.mock.calls[0]?.[0]).toContain('USDC%3AGDUKMGUGDZQK6YH');
  });

  it('throws StellarRouteApiError with isNotFound() on 404', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiError('not_found', 'Asset not found in orderbook', 404),
    );
    const err = await new StellarRouteClient({ retries: 0 })
      .getOrderbook('native', 'GHOST')
      .catch((e: unknown) => e);

    expect(isStellarRouteApiError(err)).toBe(true);
    expect((err as StellarRouteApiError).isNotFound()).toBe(true);
    expect((err as StellarRouteApiError).status).toBe(404);
    expect((err as StellarRouteApiError).code).toBe('not_found');
  });
});

// ── getQuote ──────────────────────────────────────────────────────────────────

describe('getQuote', () => {
  it('uses configurable base URL', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(sampleQuote));
    await new StellarRouteClient('https://api.example.com/').getQuote(
      'native',
      'USDC:GDUKMGUGDZQK6YH...',
      100,
    );
    expect(spy.mock.calls[0]?.[0]).toBe(
      'https://api.example.com/api/v1/quote/native/USDC%3AGDUKMGUGDZQK6YH...?quote_type=sell&amount=100',
    );
  });

  it('defaults to sell quote type', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(sampleQuote));
    await new StellarRouteClient().getQuote('native', 'USDC');
    expect(spy.mock.calls[0]?.[0] as string).toContain('quote_type=sell');
  });

  it('sends buy quote type when specified', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      ok({ ...sampleQuote, quote_type: 'buy' }),
    );
    await new StellarRouteClient().getQuote('native', 'USDC:GDUKMGUGDZQK6YH...', 55, 'buy');
    expect(spy.mock.calls[0]?.[0] as string).toContain('quote_type=buy');
    expect(spy.mock.calls[0]?.[0] as string).toContain('amount=55');
  });

  it('omits amount param when not provided', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(sampleQuote));
    await new StellarRouteClient().getQuote('native', 'USDC');
    expect(spy.mock.calls[0]?.[0] as string).not.toContain('amount=');
  });

  it('appends slippage_bps query parameter when provided', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(sampleQuote));
    await new StellarRouteClient().getQuote('native', 'USDC', 100, 'sell', 100);
    const url = new URL(spy.mock.calls[0]?.[0] as string);
    expect(url.searchParams.get('slippage_bps')).toBe('100');
  });

  it('throws StellarRouteApiError on 400 validation error', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiError('validation_error', 'Invalid amount', 400),
    );
    const client = new StellarRouteClient({ retries: 0 });
    try {
      await client.getQuote('native', 'USDC', -10);
      expect.fail('should have thrown');
    } catch (err) {
      expect(isStellarRouteApiError(err)).toBe(true);
      if (isStellarRouteApiError(err)) {
        expect(err.status).toBe(400);
        expect(err.code).toBe('validation_error');
        expect(err.isValidationError()).toBe(true);
      }
    }
  });
});

// ── getRoutes ─────────────────────────────────────────────────────────────────

describe('getRoutes', () => {
  it('returns only the path array on 200', async () => {
    const sampleRoute = {
      base_asset: NATIVE,
      quote_asset: USDC,
      amount: '100',
      path: sampleQuote.path,
      slippage_bps: 50,
      timestamp: Date.now(),
    };

    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(sampleRoute));
    const result = await new StellarRouteClient().getRoutes('native', 'USDC', 100);
    expect(Array.isArray(result)).toBe(true);
    expect(result).toHaveLength(1);
    expect(result[0]?.source).toBe('sdex');
  });

  it('calls the correct endpoint with parameters', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok({ path: [] }));
    await new StellarRouteClient().getRoutes('native', 'USDC', 100, 'buy', 100);
    const url = new URL(spy.mock.calls[0]?.[0] as string);
    expect(url.pathname).toBe('/api/v1/route/native/USDC');
    expect(url.searchParams.get('amount')).toBe('100');
    expect(url.searchParams.get('quote_type')).toBe('buy');
    expect(url.searchParams.get('slippage_bps')).toBe('100');
  });
});

// ── Error handling ────────────────────────────────────────────────────────────

describe('error handling', () => {
  it('maps 429 to isRateLimited()', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      apiError('rate_limit_exceeded', 'Too many requests', 429),
    );
    const err = await new StellarRouteClient({ retries: 0 })
      .getPairs()
      .catch((e: unknown) => e);

    expect(isStellarRouteApiError(err)).toBe(true);
    expect((err as StellarRouteApiError).isRateLimited()).toBe(true);
    expect((err as StellarRouteApiError).status).toBe(429);
  });

  it('maps 503 to isOverloaded()', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      apiError('overloaded', 'Service overloaded', 503),
    );
    const err = await new StellarRouteClient({ retries: 0 })
      .getPairs()
      .catch((e: unknown) => e);

    expect(isStellarRouteApiError(err)).toBe(true);
    expect((err as StellarRouteApiError).isOverloaded()).toBe(true);
    expect((err as StellarRouteApiError).status).toBe(503);
  });

  it('maps 422 to isStaleMarketData()', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      apiError('stale_market_data', 'Quote data stale', 422),
    );
    const err = await new StellarRouteClient({ retries: 0 })
      .getPairs()
      .catch((e: unknown) => e);

    expect(isStellarRouteApiError(err)).toBe(true);
    expect((err as StellarRouteApiError).isStaleMarketData()).toBe(true);
    expect((err as StellarRouteApiError).status).toBe(422);
  });

  it('maps network failure to isNetworkError()', async () => {
    vi.spyOn(globalThis, 'fetch').mockRejectedValue(new TypeError('Failed to fetch'));
    const err = await new StellarRouteClient({ retries: 0 })
      .getHealth()
      .catch((e: unknown) => e);

    expect(isStellarRouteApiError(err)).toBe(true);
    expect((err as StellarRouteApiError).isNetworkError()).toBe(true);
    expect((err as StellarRouteApiError).status).toBe(0);
    expect((err as StellarRouteApiError).code).toBe('network_error');
  });

  it('retries on 500 and eventually throws', async () => {
    const spy = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue(apiError('internal_error', 'Server error', 500));

    const err = await new StellarRouteClient({ retries: 2, timeoutMs: 5_000 })
      .getHealth()
      .catch((e: unknown) => e);

    // 1 initial + 2 retries = 3 total calls
    expect(spy).toHaveBeenCalledTimes(3);
    expect(isStellarRouteApiError(err)).toBe(true);
    expect((err as StellarRouteApiError).status).toBe(500);
  });

  it('does not retry on 400', async () => {
    const spy = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue(apiError('validation_error', 'Bad input', 400));

    await new StellarRouteClient({ retries: 2 }).getHealth().catch(() => {});
    // 400 is not retried — only 1 call
    expect(spy).toHaveBeenCalledTimes(1);
  });

  it('does not retry on 404', async () => {
    const spy = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue(apiError('not_found', 'Not found', 404));

    await new StellarRouteClient({ retries: 2 }).getHealth().catch(() => {});
    expect(spy).toHaveBeenCalledTimes(1);
  });

  it('preserves error details field', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          error: 'validation_error',
          message: 'Bad params',
          details: { field: 'amount', reason: 'must be positive' },
        }),
        { status: 400 },
      ),
    );
    const err = await new StellarRouteClient({ retries: 0 })
      .getHealth()
      .catch((e: unknown) => e);

    expect((err as StellarRouteApiError).details).toEqual({
      field: 'amount',
      reason: 'must be positive',
    });
  });
});

// ── isStellarRouteApiError type guard ─────────────────────────────────────────

describe('isStellarRouteApiError', () => {
  it('returns true for StellarRouteApiError instances', () => {
    const err = new StellarRouteApiError(404, 'not_found', 'Not found');
    expect(isStellarRouteApiError(err)).toBe(true);
  });

  it('returns false for plain Error', () => {
    expect(isStellarRouteApiError(new Error('oops'))).toBe(false);
  });

  it('returns false for non-error values', () => {
    expect(isStellarRouteApiError(null)).toBe(false);
    expect(isStellarRouteApiError('string')).toBe(false);
    expect(isStellarRouteApiError(42)).toBe(false);
  });
});

// ── StellarRouteClient constructor ────────────────────────────────────────────

describe('StellarRouteClient constructor', () => {
  it('accepts a plain string for backward compatibility', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(sampleHealth));
    await new StellarRouteClient('https://custom.example.com').getHealth();
    expect(spy.mock.calls[0]?.[0] as string).toContain('https://custom.example.com');
  });

  it('strips trailing slash from base URL', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(sampleHealth));
    await new StellarRouteClient({ baseUrl: 'https://api.example.com/' }).getHealth();
    expect(spy.mock.calls[0]?.[0]).toBe('https://api.example.com/health');
  });

  it('sends custom headers with every request', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(sampleHealth));
    await new StellarRouteClient({
      headers: { 'X-Api-Key': 'test-key' },
    }).getHealth();
    const init = spy.mock.calls[0]?.[1] as RequestInit;
    expect((init.headers as Record<string, string>)['X-Api-Key']).toBe('test-key');
  });
});
