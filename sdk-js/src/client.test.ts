import { describe, expect, it, vi, afterEach, beforeEach } from 'vitest';
import {
  StellarRouteClient,
  StellarRouteApiError,
  isStellarRouteApiError,
  parseApiErrorBody,
} from './client.js';
import type {
  BatchOrderbookResponse,
  BatchQuoteResponse,
  HealthStatus,
  Orderbook,
  PairsResponse,
  PriceQuote,
} from './types.js';

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

const sampleBatchQuote: BatchQuoteResponse = {
  quotes: [sampleQuote, sampleQuote],
  total: 2,
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

// ── getOrderbooksBatch ──────────────────────────────────────────────────────────

const sampleOrderbookBatch: BatchOrderbookResponse = {
  results: [
    { index: 0, status: 'ok', orderbook: sampleOrderbook },
    {
      index: 1,
      status: 'error',
      error: { code: 'not_found', message: 'Asset not found in orderbook' },
    },
  ],
  items_succeeded: 1,
  items_failed: 1,
  total: 2,
};

describe('getOrderbooksBatch', () => {
  it('POSTs a typed payload to /api/v1/batch/orderbook', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(sampleOrderbookBatch));
    const result = await new StellarRouteClient().getOrderbooksBatch([
      { base: 'native', quote: 'USDC' },
      { base: 'native', quote: 'GHOST' },
    ]);

    expect(spy.mock.calls[0]?.[0]).toBe('http://localhost:8080/api/v1/batch/orderbook');
    const init = spy.mock.calls[0]?.[1];
    expect(init?.method).toBe('POST');
    expect(JSON.parse(init?.body as string)).toEqual({
      requests: [
        { base: 'native', quote: 'USDC' },
        { base: 'native', quote: 'GHOST' },
      ],
    });

    expect(result.total).toBe(2);
    expect(result.items_succeeded).toBe(1);
    expect(result.results[0]?.orderbook?.asks[0]?.price).toBe('0.1060000');
    expect(result.results[1]?.error?.code).toBe('not_found');
  });

  it('maps an API error to StellarRouteApiError', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiError('validation_error', 'Batch request must contain at least 1 item', 400),
    );
    const err = await new StellarRouteClient({ retries: 0 })
      .getOrderbooksBatch([])
      .catch((e: unknown) => e);

    expect(isStellarRouteApiError(err)).toBe(true);
    expect((err as StellarRouteApiError).status).toBe(400);
    expect((err as StellarRouteApiError).code).toBe('validation_error');
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

// ── getRankedRoutes ─────────────────────────────────────────────────────────

describe('getRankedRoutes', () => {
  const sampleRankedRoutes = {
    base_asset: NATIVE,
    quote_asset: USDC,
    amount: '100',
    routes: [
      {
        estimated_output: '10.5000000',
        impact_bps: 12,
        score: 0.95,
        policy_used: 'production',
        path: [
          {
            from_asset: NATIVE,
            to_asset: USDC,
            price: '0.1050000',
            amount_out_of_hop: '10.5000000',
            fee_bps: 30,
            source: 'sdex',
          },
        ],
      },
      {
        estimated_output: '10.4000000',
        impact_bps: 25,
        score: 0.88,
        policy_used: 'production',
        path: [
          {
            from_asset: NATIVE,
            to_asset: USDC,
            price: '0.1040000',
            amount_out_of_hop: '10.4000000',
            fee_bps: 30,
            source: 'amm:pool1',
          },
        ],
      },
    ],
    timestamp: Date.now(),
  };

  it('returns typed RankedRoutesResponse on 200', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(sampleRankedRoutes));
    const result = await new StellarRouteClient().getRankedRoutes('native', 'USDC', 100);
    expect(result.routes).toHaveLength(2);
    expect(result.routes[0]?.score).toBe(0.95);
    expect(result.routes[0]?.estimated_output).toBe('10.5000000');
    expect(result.routes[0]?.path[0]?.source).toBe('sdex');
    expect(result.amount).toBe('100');
  });

  it('calls the correct /api/v1/routes endpoint', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(sampleRankedRoutes));
    await new StellarRouteClient().getRankedRoutes('native', 'USDC', 100);
    const url = new URL(spy.mock.calls[0]?.[0] as string);
    expect(url.pathname).toBe('/api/v1/routes/native/USDC');
    expect(url.searchParams.get('amount')).toBe('100');
  });

  it('appends limit and max_hops query params when provided', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(sampleRankedRoutes));
    await new StellarRouteClient().getRankedRoutes('native', 'USDC', 100, 10, 4);
    const url = new URL(spy.mock.calls[0]?.[0] as string);
    expect(url.searchParams.get('limit')).toBe('10');
    expect(url.searchParams.get('max_hops')).toBe('4');
  });

  it('omits optional params when not provided', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(sampleRankedRoutes));
    await new StellarRouteClient().getRankedRoutes('native', 'USDC');
    const url = new URL(spy.mock.calls[0]?.[0] as string);
    expect(url.searchParams.has('amount')).toBe(false);
    expect(url.searchParams.has('limit')).toBe(false);
    expect(url.searchParams.has('max_hops')).toBe(false);
  });

  it('URL-encodes asset identifiers with colons', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(sampleRankedRoutes));
    await new StellarRouteClient().getRankedRoutes('native', 'USDC:GDUKMGUGDZQK6YH...', 100);
    expect(spy.mock.calls[0]?.[0]).toContain('USDC%3AGDUKMGUGDZQK6YH');
  });

  it('throws StellarRouteApiError on 404', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiError('not_found', 'No route found', 404),
    );
    const err = await new StellarRouteClient({ retries: 0 })
      .getRankedRoutes('native', 'GHOST')
      .catch((e: unknown) => e);

    expect(isStellarRouteApiError(err)).toBe(true);
    expect((err as StellarRouteApiError).isNotFound()).toBe(true);
  });
});

// ── getPriceHistory ─────────────────────────────────────────────────────────

describe('getPriceHistory', () => {
  const samplePriceHistory = {
    base_asset: NATIVE,
    quote_asset: USDC,
    window: '24h',
    source: 'orderbook_snapshots.mid_price',
    generated_at: 1_717_171_717,
    points: [
      { timestamp: 1_717_164_400, price: '0.1050000' },
      { timestamp: 1_717_168_000, price: '0.1055000' },
      { timestamp: 1_717_171_600, price: '0.1060000' },
    ],
  };

  it('returns typed PriceHistoryResponse on 200', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(samplePriceHistory));
    const result = await new StellarRouteClient().getPriceHistory('native', 'USDC');
    expect(result.window).toBe('24h');
    expect(result.source).toBe('orderbook_snapshots.mid_price');
    expect(result.points).toHaveLength(3);
    expect(result.points[0]?.price).toBe('0.1050000');
  });

  it('calls the correct endpoint', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(samplePriceHistory));
    await new StellarRouteClient().getPriceHistory('native', 'USDC');
    const url = new URL(spy.mock.calls[0]?.[0] as string);
    expect(url.pathname).toBe('/api/v1/price-history/native/USDC');
  });

  it('appends window query param when provided', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(samplePriceHistory));
    await new StellarRouteClient().getPriceHistory('native', 'USDC', { window: '7d' });
    const url = new URL(spy.mock.calls[0]?.[0] as string);
    expect(url.searchParams.get('window')).toBe('7d');
  });

  it('omits window param when not provided', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(samplePriceHistory));
    await new StellarRouteClient().getPriceHistory('native', 'USDC');
    const url = new URL(spy.mock.calls[0]?.[0] as string);
    expect(url.searchParams.has('window')).toBe(false);
  });

  it('URL-encodes asset identifiers with colons', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(samplePriceHistory));
    await new StellarRouteClient().getPriceHistory('native', 'USDC:GDUKMGUGDZQK6YH...');
    expect(spy.mock.calls[0]?.[0]).toContain('USDC%3AGDUKMGUGDZQK6YH');
  });

  it('throws StellarRouteApiError on 404 pair-not-found', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiError('not_found', 'Pair not found', 404),
    );
    const err = await new StellarRouteClient({ retries: 0 })
      .getPriceHistory('native', 'GHOST')
      .catch((e: unknown) => e);

    expect(isStellarRouteApiError(err)).toBe(true);
    expect((err as StellarRouteApiError).isNotFound()).toBe(true);
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

// ── simulateRoute ─────────────────────────────────────────────────────────────

const sampleSimulateRequest = {
  route: {
    hops: [{ from_asset: 'native', to_asset: 'USDC:GDUKMGUGDZQK6YH...', source: 'sdex' }],
  },
  amount: '100',
  slippage_bps: 50,
};

const sampleSimulateResponse = {
  quote: sampleQuote,
  exclusion_diagnostics: { excluded_venues: [] },
};

describe('simulateRoute', () => {
  it('POSTs to /api/v1/simulate/route with the request body', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(sampleSimulateResponse));
    const result = await new StellarRouteClient().simulateRoute(sampleSimulateRequest);

    expect(spy.mock.calls[0]?.[0]).toBe('http://localhost:8080/api/v1/simulate/route');
    const init = spy.mock.calls[0]?.[1];
    expect(init?.method).toBe('POST');
    expect(JSON.parse(init?.body as string)).toEqual(sampleSimulateRequest);
    expect(result.quote.price).toBe('0.99');
  });

  it('returns exclusion_diagnostics when present', async () => {
    const withDiag = {
      ...sampleSimulateResponse,
      exclusion_diagnostics: {
        excluded_venues: [{ venue_ref: 'amm:pool1', reason: 'stale_data' }],
      },
    };
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(withDiag));
    const result = await new StellarRouteClient().simulateRoute(sampleSimulateRequest);
    expect(result.exclusion_diagnostics?.excluded_venues[0]?.venue_ref).toBe('amm:pool1');
  });

  it('throws StellarRouteApiError on 404', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiError('not_found', 'No route found', 404),
    );
    const err = await new StellarRouteClient({ retries: 0 })
      .simulateRoute(sampleSimulateRequest)
      .catch((e: unknown) => e);
    expect(isStellarRouteApiError(err)).toBe(true);
    expect((err as StellarRouteApiError).isNotFound()).toBe(true);
  });

  it('throws StellarRouteApiError on 400 validation error', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiError('validation_error', 'Invalid hops', 400),
    );
    const err = await new StellarRouteClient({ retries: 0 })
      .simulateRoute({ route: { hops: [] }, amount: '0' })
      .catch((e: unknown) => e);
    expect(isStellarRouteApiError(err)).toBe(true);
    expect((err as StellarRouteApiError).isValidationError()).toBe(true);
  });
});

// ── prepareSwap / submitSwap / executeSwap / confirmSwap ─────────────────────

function enveloped<T>(data: T) {
  return {
    v: 1,
    timestamp: 1_740_312_000_000,
    request_id: 'req-test',
    data,
  };
}

function apiErrorEnveloped(
  code: string,
  message: string,
  status: number,
  details?: unknown,
): Response {
  return ok(
    enveloped({
      error: code,
      message,
      ...(details !== undefined ? { details } : {}),
    }),
    status,
  );
}

const classicPrepare = {
  quote_id: 'q123',
  xdr_envelope: 'opaque_unsigned_xdr',
  expected_output: '98',
  min_output: '97',
  expires_at: 1234567890,
  execution_mode: 'classic_path_payment' as const,
  network_passphrase: 'Test SDF Network ; September 2015',
};

describe('parseApiErrorBody', () => {
  it('parses flat and nested { data: { error, message, details } } forms', () => {
    expect(
      parseApiErrorBody({
        error: 'quote_expired',
        message: 'expired',
        details: { status: 'expired' },
      }),
    ).toEqual({
      error: 'quote_expired',
      message: 'expired',
      details: { status: 'expired' },
    });
    expect(
      parseApiErrorBody(
        enveloped({
          error: 'duplicate_quote',
          message: 'conflict',
          details: { status: 'active_prepare_exists' },
        }),
      ),
    ).toEqual({
      error: 'duplicate_quote',
      message: 'conflict',
      details: { status: 'active_prepare_exists' },
    });
  });
});

describe('nested API error envelope', () => {
  it.each([
    ['unsupported_route', 422, { hops: 2 }],
    ['unsupported_execution_mode', 422, { source: 'amm' }],
    ['quote_expired', 422, { quote_id: 'q1' }],
    ['dependency_unavailable', 503, { dependency: 'horizon' }],
  ] as const)(
    'preserves code/details/status for %s',
    async (code, status, details) => {
      vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
        apiErrorEnveloped(code, `${code} message`, status, details),
      );

      const err = await new StellarRouteClient({ retries: 0 })
        .prepareSwap({
          route: sampleSimulateRequest.route,
          amount: '1',
          sender: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
        })
        .catch((e: unknown) => e);

      expect(isStellarRouteApiError(err)).toBe(true);
      expect((err as StellarRouteApiError).code).toBe(code);
      expect((err as StellarRouteApiError).status).toBe(status);
      expect((err as StellarRouteApiError).details).toEqual(details);
      expect((err as StellarRouteApiError).message).toBe(`${code} message`);
    },
  );

  it('preserves conflict details.status for active_prepare_exists', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiErrorEnveloped('duplicate_quote', 'active prepare', 409, {
        status: 'active_prepare_exists',
        quote_id: '',
      }),
    );

    const err = await new StellarRouteClient({ retries: 0 })
      .prepareSwap({
        route: sampleSimulateRequest.route,
        amount: '1',
        sender: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
      })
      .catch((e: unknown) => e);

    expect((err as StellarRouteApiError).status).toBe(409);
    expect((err as StellarRouteApiError).code).toBe('duplicate_quote');
    expect((err as StellarRouteApiError).details).toMatchObject({
      status: 'active_prepare_exists',
    });
  });
});

describe('prepareSwap', () => {
  it('POSTs to /api/v1/swap/prepare and unwraps ApiResponse.data', async () => {
    const spy = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValueOnce(ok(enveloped(classicPrepare)));

    const result = await new StellarRouteClient({ retries: 0 }).prepareSwap({
      route: sampleSimulateRequest.route,
      amount: '100',
      sender: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
      slippage_bps: 50,
    });

    expect(spy.mock.calls[0]?.[0]).toContain('/api/v1/swap/prepare');
    expect(result).toEqual(classicPrepare);
  });
});

describe('submitSwap', () => {
  it('POSTs to /api/v1/swap/submit and unwraps ApiResponse.data', async () => {
    const payload = {
      quote_id: 'q123',
      tx_hash: 'tx456',
      status: 'pending',
    };
    const spy = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValueOnce(ok(enveloped(payload)));

    const result = await new StellarRouteClient({ retries: 0 }).submitSwap({
      quote_id: 'q123',
      signed_xdr: 'signed_xdr',
    });

    expect(spy.mock.calls[0]?.[0]).toContain('/api/v1/swap/submit');
    const body = JSON.parse((spy.mock.calls[0]?.[1] as RequestInit).body as string);
    expect(body).toEqual({ quote_id: 'q123', signed_xdr: 'signed_xdr' });
    expect(result).toEqual(payload);
  });
});

describe('executeSwap', () => {
  const TESTNET_PASSPHRASE = 'Test SDF Network ; September 2015';

  it('classic happy path: prepare → sign server XDR → submit', async () => {
    const fetchSpy = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValueOnce(ok(enveloped(classicPrepare)))
      .mockResolvedValueOnce(
        ok(
          enveloped({
            quote_id: 'q123',
            tx_hash: 'tx456',
            status: 'pending',
          }),
        ),
      );

    const signTransaction = vi.fn().mockResolvedValue('opaque_signed_xdr');

    const result = await new StellarRouteClient({ retries: 0 }).executeSwap({
      route: sampleSimulateRequest.route,
      amount: '100',
      sender: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
      slippage_bps: 50,
      networkPassphrase: TESTNET_PASSPHRASE,
      signTransaction,
    });

    expect(fetchSpy).toHaveBeenCalledTimes(2);
    expect(signTransaction).toHaveBeenCalledWith('opaque_unsigned_xdr');
    expect(result).toMatchObject({
      quote_id: 'q123',
      execution_mode: 'classic_path_payment',
      expected_output: '98',
      min_output: '97',
      tx_hash: 'tx456',
    });
  });

  it('accepts networkPassphrase callback when it matches prepare', async () => {
    vi.spyOn(globalThis, 'fetch')
      .mockResolvedValueOnce(ok(enveloped(classicPrepare)))
      .mockResolvedValueOnce(
        ok(
          enveloped({
            quote_id: 'q123',
            tx_hash: 'tx456',
            status: 'pending',
          }),
        ),
      );
    const signTransaction = vi.fn().mockResolvedValue('opaque_signed_xdr');
    const getPassphrase = vi.fn().mockResolvedValue(TESTNET_PASSPHRASE);

    await new StellarRouteClient({ retries: 0 }).executeSwap({
      route: sampleSimulateRequest.route,
      amount: '100',
      sender: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
      networkPassphrase: getPassphrase,
      signTransaction,
    });

    expect(getPassphrase).toHaveBeenCalled();
    expect(signTransaction).toHaveBeenCalledTimes(1);
  });

  it('returns network_mismatch without signing or submitting on passphrase mismatch', async () => {
    const fetchSpy = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValueOnce(ok(enveloped(classicPrepare)));
    const signTransaction = vi.fn();

    const err = await new StellarRouteClient({ retries: 0 })
      .executeSwap({
        route: sampleSimulateRequest.route,
        amount: '100',
        sender: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
        networkPassphrase: 'Public Global Stellar Network ; September 2015',
        signTransaction,
      })
      .catch((e: unknown) => e);

    expect(isStellarRouteApiError(err)).toBe(true);
    expect((err as StellarRouteApiError).code).toBe('network_mismatch');
    expect((err as StellarRouteApiError).details).toMatchObject({
      status: 'network_mismatch',
    });
    expect(signTransaction).not.toHaveBeenCalled();
    expect(
      fetchSpy.mock.calls.filter((c) => String(c[0]).includes('/swap/submit')),
    ).toHaveLength(0);
  });

  it('refuses to sign non-classic execution_mode', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      ok(
        enveloped({
          ...classicPrepare,
          execution_mode: 'soroban_router',
        }),
      ),
    );
    const signTransaction = vi.fn();
    const err = await new StellarRouteClient({ retries: 0 })
      .executeSwap({
        route: sampleSimulateRequest.route,
        amount: '100',
        sender: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
        networkPassphrase: TESTNET_PASSPHRASE,
        signTransaction,
      })
      .catch((e: unknown) => e);

    expect((err as StellarRouteApiError).code).toBe('unsupported_execution_mode');
    expect(signTransaction).not.toHaveBeenCalled();
  });

  it('ambiguous submit retries reuse the exact same body without re-prepare/sign', async () => {
    const fetchSpy = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValueOnce(ok(enveloped(classicPrepare)))
      .mockResolvedValueOnce(
        apiErrorEnveloped('dependency_unavailable', 'Horizon down', 503),
      )
      .mockResolvedValueOnce(
        ok(
          enveloped({
            quote_id: 'q123',
            tx_hash: 'tx456',
            status: 'pending',
          }),
        ),
      );

    const signTransaction = vi.fn().mockResolvedValue('opaque_signed_xdr');
    await new StellarRouteClient({ retries: 0 }).executeSwap({
      route: sampleSimulateRequest.route,
      amount: '100',
      sender: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
      networkPassphrase: TESTNET_PASSPHRASE,
      signTransaction,
      ambiguousSubmitRetries: 2,
    });

    expect(signTransaction).toHaveBeenCalledTimes(1);
    const prepareCalls = fetchSpy.mock.calls.filter((c) =>
      String(c[0]).includes('/swap/prepare'),
    );
    const submitCalls = fetchSpy.mock.calls.filter((c) =>
      String(c[0]).includes('/swap/submit'),
    );
    expect(prepareCalls).toHaveLength(1);
    expect(submitCalls).toHaveLength(2);
    const body1 = JSON.parse((submitCalls[0]?.[1] as RequestInit).body as string);
    const body2 = JSON.parse((submitCalls[1]?.[1] as RequestInit).body as string);
    expect(body1).toEqual(body2);
    expect(body1).toEqual({
      quote_id: 'q123',
      signed_xdr: 'opaque_signed_xdr',
    });
  });

  it('propagates prepare failure without calling sign', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiErrorEnveloped('validation_error', 'Invalid sender', 400),
    );

    const signTransaction = vi.fn();
    const err = await new StellarRouteClient({ retries: 0 })
      .executeSwap({
        route: sampleSimulateRequest.route,
        amount: '100',
        sender: 'invalid',
        networkPassphrase: TESTNET_PASSPHRASE,
        signTransaction,
      })
      .catch((e: unknown) => e);

    expect(isStellarRouteApiError(err)).toBe(true);
    expect(signTransaction).not.toHaveBeenCalled();
    expect((err as StellarRouteApiError).code).toBe('validation_error');
  });
});

describe('confirmSwap', () => {
  it('polls Horizon until the transaction is found', async () => {
    const fetchSpy = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValueOnce(ok({}, 404))
      .mockResolvedValueOnce(
        ok({
          hash: 'tx456',
          successful: true,
          ledger: 12345,
        }),
      );

    const result = await new StellarRouteClient({ retries: 0 }).confirmSwap(
      'tx456',
      {
        horizonUrl: 'https://horizon-testnet.stellar.org',
        pollIntervalMs: 1,
        timeoutMs: 1_000,
        expectedTxHash: 'tx456',
      },
    );

    expect(fetchSpy).toHaveBeenCalledTimes(2);
    expect(result).toEqual({
      tx_hash: 'tx456',
      successful: true,
      ledger: 12345,
      horizon_url: 'https://horizon-testnet.stellar.org/transactions/tx456',
    });
  });

  it('throws when Horizon never returns the transaction', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(ok({}, 404));

    const err = await new StellarRouteClient({ retries: 0 })
      .confirmSwap('missing', {
        horizonUrl: 'https://horizon-testnet.stellar.org',
        pollIntervalMs: 1,
        timeoutMs: 5,
      })
      .catch((e: unknown) => e);

    expect(isStellarRouteApiError(err)).toBe(true);
    expect((err as StellarRouteApiError).code).toBe('network_error');
    expect((err as StellarRouteApiError).details).toMatchObject({
      status: 'confirm_timeout',
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
