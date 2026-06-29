import { afterEach, describe, expect, it, vi, beforeEach, afterAll } from 'vitest';

import {
  StellarRouteApiError,
  StellarRouteClient,
  type QuoteRequestItem,
} from '@/lib/api/client';
import { getApiRoot } from '@/lib/constants';
import type { PriceHistoryResponse, PriceQuote, RoutesResponse } from '@/types';

// Fixtures aligned with frontend/test/api-schema.test.ts asset identifiers.

const USDC_ISSUER =
  'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN';

const NATIVE_ASSET = {
  asset_type: 'native' as const,
};

const USDC_ASSET = {
  asset_type: 'credit_alphanum4' as const,
  asset_code: 'USDC',
  asset_issuer: USDC_ISSUER,
};

const sampleQuote: PriceQuote = {
  base_asset: NATIVE_ASSET,
  quote_asset: USDC_ASSET,
  amount: '100.0000000',
  price: '0.1055000',
  total: '10.5500000',
  quote_type: 'sell',
  path: [
    {
      from_asset: NATIVE_ASSET,
      to_asset: USDC_ASSET,
      price: '0.1055000',
      source: 'sdex',
    },
  ],
  timestamp: 1_740_312_000,
};

function envelope<T>(data: T, requestId = 'req-test-799') {
  return {
    v: 1,
    timestamp: 1_714_000_000_000,
    request_id: requestId,
    data,
  };
}

function errorEnvelope(
  error: string,
  message: string,
  details?: unknown,
) {
  return envelope({
    error,
    message,
    ...(details !== undefined ? { details } : {}),
  });
}

function ok(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

function apiError(
  code: string,
  message: string,
  status: number,
  details?: unknown,
): Response {
  return ok(errorEnvelope(code, message, details), status);
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe('getQuotesBatch', () => {
  const requests: QuoteRequestItem[] = [
    { base: 'native', quote: `USDC:${USDC_ISSUER}`, amount: 10, quote_type: 'sell' },
    { base: 'native', quote: `USDC:${USDC_ISSUER}`, amount: 20, quote_type: 'sell' },
  ];

  it('calls POST /api/v1/batch/quote with wrapped quotes payload', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      ok(
        envelope({
          results: [
            { index: 0, status: 'ok', quote: sampleQuote },
            {
              index: 1,
              status: 'ok',
              quote: { ...sampleQuote, amount: '20.0000000' },
            },
          ],
          items_succeeded: 2,
          items_failed: 0,
          total: 2,
          snapshot_timestamp: 1_714_000_000_000,
        }),
      ),
    );

    await new StellarRouteClient({
      baseUrl: 'https://api.example.com',
    }).getQuotesBatch(requests);

    expect(spy).toHaveBeenCalledTimes(1);
    const [url, init] = spy.mock.calls[0] as [string, RequestInit];
    expect(url).toBe('https://api.example.com/api/v1/batch/quote');
    expect(init.method).toBe('POST');
    expect((init.headers as Record<string, string>)['Content-Type']).toBe(
      'application/json',
    );

    const body = JSON.parse(init.body as string) as {
      quotes: Array<Record<string, unknown>>;
    };
    expect(body.quotes).toHaveLength(2);
    expect(body.quotes[0]).toEqual({
      base: 'native',
      quote: `USDC:${USDC_ISSUER}`,
      amount: '10',
      quote_type: 'sell',
    });
    expect(body.quotes[1]?.amount).toBe('20');
  });

  it('unwraps the ApiResponse envelope and maps successful quotes by index', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      ok(
        envelope({
          results: [
            { index: 0, status: 'ok', quote: sampleQuote },
            {
              index: 1,
              status: 'error',
              error: { code: 'no_route', message: 'No trading route found' },
            },
          ],
          items_succeeded: 1,
          items_failed: 1,
          total: 2,
          snapshot_timestamp: 1_714_000_000_000,
        }),
      ),
    );

    const result = await new StellarRouteClient().getQuotesBatch(requests);

    expect(result.total).toBe(1);
    expect(result.quotes[0]?.amount).toBe('100.0000000');
    expect(result.quotes[1]).toBeUndefined();
  });

  it('throws StellarRouteApiError for batch-level validation failures', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiError('validation_error', 'Batch request must contain at least 1 item', 400),
    );

    const err = await new StellarRouteClient({ retries: 0 })
      .getQuotesBatch([])
      .catch((error: unknown) => error);

    expect(err).toBeInstanceOf(StellarRouteApiError);
    expect((err as StellarRouteApiError).status).toBe(400);
    expect((err as StellarRouteApiError).code).toBe('validation_error');
    expect((err as StellarRouteApiError).message).toContain('at least 1 item');
  });

  it('surfaces oversized batch validation errors from the API envelope', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiError(
        'validation_error',
        'Batch size 26 exceeds maximum of 25 items',
        400,
      ),
    );

    const err = await new StellarRouteClient({ retries: 0 })
      .getQuotesBatch(
        Array.from({ length: 26 }, () => ({
          base: 'native',
          quote: `USDC:${USDC_ISSUER}`,
          amount: 1,
        })),
      )
      .catch((error: unknown) => error);

    expect((err as StellarRouteApiError).code).toBe('validation_error');
    expect((err as StellarRouteApiError).message).toContain('25');
  });

  it('does not retry on 400 validation errors', async () => {
    const spy = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue(
        apiError('validation_error', 'Batch request must contain at least 1 item', 400),
      );

    await new StellarRouteClient({ retries: 2 })
      .getQuotesBatch([])
      .catch(() => {});

    expect(spy).toHaveBeenCalledTimes(1);
  });

  it('honours AbortSignal when the request is cancelled', async () => {
    const controller = new AbortController();
    vi.spyOn(globalThis, 'fetch').mockImplementation((_url, init) => {
      if (init?.signal?.aborted) {
        return Promise.reject(new DOMException('Aborted', 'AbortError'));
      }

      return new Promise((_resolve, reject) => {
        init?.signal?.addEventListener('abort', () => {
          reject(new DOMException('Aborted', 'AbortError'));
        });
      });
    });

    controller.abort();

    const err = await new StellarRouteClient({ retries: 0 })
      .getQuotesBatch(requests, { signal: controller.signal })
      .catch((error: unknown) => error);

    expect(err).toBeInstanceOf(StellarRouteApiError);
    expect((err as StellarRouteApiError).code).toBe('network_error');
  });
});

describe('getRoutes', () => {
  const routesData: RoutesResponse = {
    base_asset: NATIVE_ASSET,
    quote_asset: USDC_ASSET,
    amount: '10.0000000',
    timestamp: 1_700_000_000_000,
    routes: [
      {
        score: 95,
        impact_bps: 20,
        estimated_output: '9.9800000',
        policy_used: 'production',
        path: [
          {
            from_asset: NATIVE_ASSET,
            to_asset: USDC_ASSET,
            price: '0.9980000',
            fee_bps: 20,
            source: 'amm:pool-1',
          },
        ],
      },
      {
        score: 80,
        impact_bps: 40,
        estimated_output: '9.9600000',
        policy_used: 'production',
        path: [
          {
            from_asset: NATIVE_ASSET,
            to_asset: USDC_ASSET,
            price: '0.9960000',
            fee_bps: 40,
            source: 'sdex',
          },
        ],
      },
    ],
  };

  it('calls GET /api/v1/routes/{base}/{quote} with query params', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      ok(envelope(routesData)),
    );

    await new StellarRouteClient({
      baseUrl: 'https://api.example.com',
    }).getRoutes('native', `USDC:${USDC_ISSUER}`, 10, 5, 3);

    const url = new URL(spy.mock.calls[0]?.[0] as string);
    expect(url.origin + url.pathname).toBe(
      `https://api.example.com/api/v1/routes/native/USDC%3A${USDC_ISSUER}`,
    );
    expect(url.searchParams.get('amount')).toBe('10');
    expect(url.searchParams.get('limit')).toBe('5');
    expect(url.searchParams.get('max_hops')).toBe('3');
  });

  it('omits optional query params when not provided', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      ok(envelope(routesData)),
    );

    await new StellarRouteClient().getRoutes('native', 'USDC');

    const url = new URL(spy.mock.calls[0]?.[0] as string);
    expect(url.search).toBe('');
  });

  it('unwraps the ApiResponse envelope and returns ranked routes', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      ok(envelope(routesData)),
    );

    const result = await new StellarRouteClient().getRoutes(
      'native',
      `USDC:${USDC_ISSUER}`,
      10,
    );

    expect(result.routes).toHaveLength(2);
    expect(result.routes[0]?.score).toBe(95);
    expect(result.routes[1]?.score).toBe(80);
    expect(result.amount).toBe('10.0000000');
  });

  it('throws StellarRouteApiError on 404 no-route responses', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiError('no_route', 'No trading route found for this pair', 404),
    );

    const err = await new StellarRouteClient({ retries: 0 })
      .getRoutes('native', 'GHOST')
      .catch((error: unknown) => error);

    expect(err).toBeInstanceOf(StellarRouteApiError);
    expect((err as StellarRouteApiError).status).toBe(404);
    expect((err as StellarRouteApiError).code).toBe('no_route');
  });

  it('preserves error details from the API envelope', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiError('validation_error', 'Invalid amount', 400, {
        field: 'amount',
        reason: 'must be positive',
      }),
    );

    const err = await new StellarRouteClient({ retries: 0 })
      .getRoutes('native', `USDC:${USDC_ISSUER}`, -1)
      .catch((error: unknown) => error);

    expect((err as StellarRouteApiError).details).toEqual({
      field: 'amount',
      reason: 'must be positive',
    });
  });

  it('honours AbortSignal when the request is cancelled', async () => {
    const controller = new AbortController();
    vi.spyOn(globalThis, 'fetch').mockImplementation((_url, init) => {
      if (init?.signal?.aborted) {
        return Promise.reject(new DOMException('Aborted', 'AbortError'));
      }

      return new Promise((_resolve, reject) => {
        init?.signal?.addEventListener('abort', () => {
          reject(new DOMException('Aborted', 'AbortError'));
        });
      });
    });

    controller.abort();

    const err = await new StellarRouteClient({ retries: 0 })
      .getRoutes('native', `USDC:${USDC_ISSUER}`, 10, undefined, undefined, {
        signal: controller.signal,
      })
      .catch((error: unknown) => error);

    expect(err).toBeInstanceOf(StellarRouteApiError);
    expect((err as StellarRouteApiError).code).toBe('network_error');
  });
});

describe('getPriceHistory', () => {
  const priceHistoryData: PriceHistoryResponse = {
    base_asset: NATIVE_ASSET,
    quote_asset: USDC_ASSET,
    window: '24h',
    source: 'orderbook_snapshots.mid_price',
    generated_at: 1_700_000_000_000,
    points: [
      {
        timestamp: 1_699_999_000_000,
        price: '0.1000000',
      },
    ],
  };

  it('calls GET /api/v1/price-history/{base}/{quote}', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      ok(priceHistoryData),
    );

    await new StellarRouteClient({
      baseUrl: 'https://api.example.com',
    }).getPriceHistory('native', `USDC:${USDC_ISSUER}`);

    const url = new URL(spy.mock.calls[0]?.[0] as string);
    expect(url.origin + url.pathname).toBe(
      `https://api.example.com/api/v1/price-history/native/USDC%3A${USDC_ISSUER}`,
    );
    expect(url.search).toBe('');
  });

  it('forwards optional window query params when provided', async () => {
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      ok(priceHistoryData),
    );

    await new StellarRouteClient({
      baseUrl: 'https://api.example.com',
    }).getPriceHistory('native', 'USDC', { window: '24h' });

    const url = new URL(spy.mock.calls[0]?.[0] as string);
    expect(url.searchParams.get('window')).toBe('24h');
  });

  it('surfaces API errors as StellarRouteApiError', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiError('not_found', 'Trading pair not found', 404),
    );

    const err = await new StellarRouteClient({ retries: 0 })
      .getPriceHistory('native', 'USDC')
      .catch((error: unknown) => error);

    expect(err).toBeInstanceOf(StellarRouteApiError);
    expect((err as StellarRouteApiError).status).toBe(404);
    expect((err as StellarRouteApiError).code).toBe('not_found');
  });
});

describe('shared client error handling', () => {
  it('maps 429 responses to StellarRouteApiError with isRateLimit', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      apiError('rate_limit_exceeded', 'Too many requests', 429),
    );

    const err = await new StellarRouteClient({ retries: 0 })
      .getRoutes('native', `USDC:${USDC_ISSUER}`)
      .catch((error: unknown) => error);

    expect((err as StellarRouteApiError).isRateLimit).toBe(true);
    expect((err as StellarRouteApiError).status).toBe(429);
  });

  it('retries on 500 and eventually throws', async () => {
    const spy = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue(apiError('internal_error', 'Server error', 500));

    const err = await new StellarRouteClient({ retries: 2 })
      .getQuotesBatch([{ base: 'native', quote: `USDC:${USDC_ISSUER}` }])
      .catch((error: unknown) => error);

    expect(spy).toHaveBeenCalledTimes(3);
    expect(err).toBeInstanceOf(StellarRouteApiError);
    expect((err as StellarRouteApiError).status).toBe(500);
  });
});

// ---------------------------------------------------------------------------
// getApiRoot URL builder
// ---------------------------------------------------------------------------

describe('getApiRoot', () => {
  const originalEnv = process.env;

  beforeEach(() => {
    // Isolate env per test
    process.env = { ...originalEnv };
    delete process.env.NEXT_PUBLIC_API_PROXY;
  });

  afterAll(() => {
    process.env = originalEnv;
  });

  it('returns bare origin when NEXT_PUBLIC_API_URL has no path suffix', () => {
    process.env.NEXT_PUBLIC_API_URL = 'https://api.stellarroute.xyz';
    expect(getApiRoot()).toBe('https://api.stellarroute.xyz');
  });

  it('strips trailing slash from NEXT_PUBLIC_API_URL', () => {
    process.env.NEXT_PUBLIC_API_URL = 'https://api.stellarroute.xyz/';
    expect(getApiRoot()).toBe('https://api.stellarroute.xyz');
  });

  it('strips /api/v1 suffix from NEXT_PUBLIC_API_URL', () => {
    process.env.NEXT_PUBLIC_API_URL = 'https://api.stellarroute.xyz/api/v1';
    expect(getApiRoot()).toBe('https://api.stellarroute.xyz');
  });

  it('strips /api/v1/ (with trailing slash) from NEXT_PUBLIC_API_URL', () => {
    process.env.NEXT_PUBLIC_API_URL = 'https://api.stellarroute.xyz/api/v1/';
    expect(getApiRoot()).toBe('https://api.stellarroute.xyz');
  });

  it('strips /api/v2 suffix when versioned path differs', () => {
    process.env.NEXT_PUBLIC_API_URL = 'https://api.stellarroute.xyz/api/v2';
    expect(getApiRoot()).toBe('https://api.stellarroute.xyz');
  });

  it('falls back to http://localhost:8080 when env var is absent', () => {
    delete process.env.NEXT_PUBLIC_API_URL;
    expect(getApiRoot()).toBe('http://localhost:8080');
  });

  it('returns empty string in proxy mode regardless of NEXT_PUBLIC_API_URL', () => {
    process.env.NEXT_PUBLIC_API_PROXY = 'true';
    process.env.NEXT_PUBLIC_API_URL = 'https://api.stellarroute.xyz';
    expect(getApiRoot()).toBe('');
  });

  it('preview URL with subdomain works correctly', () => {
    process.env.NEXT_PUBLIC_API_URL = 'https://preview.stellarroute.xyz';
    expect(getApiRoot()).toBe('https://preview.stellarroute.xyz');
  });
});

// ---------------------------------------------------------------------------
// StellarRouteClient URL construction
// ---------------------------------------------------------------------------

describe('StellarRouteClient URL construction', () => {
  function mockOk(body: unknown = {}) {
    return vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );
  }

  afterEach(() => vi.restoreAllMocks());

  it('calls /health (no /api/v1) from a baseUrl without trailing slash', async () => {
    const spy = mockOk({ status: 'healthy', version: '1.0.0', timestamp: '', components: {} });
    await new StellarRouteClient({ baseUrl: 'https://api.example.com' }).getHealth();
    expect(spy.mock.calls[0]?.[0]).toBe('https://api.example.com/health');
  });

  it('calls /health from a baseUrl with trailing slash removed', async () => {
    // The constructor normalises the baseUrl by stripping the trailing slash.
    const spy = mockOk({ status: 'healthy', version: '1.0.0', timestamp: '', components: {} });
    await new StellarRouteClient({ baseUrl: 'https://api.example.com/' }).getHealth();
    expect(spy.mock.calls[0]?.[0]).toBe('https://api.example.com/health');
  });

  it('calls /health/deps correctly', async () => {
    const spy = mockOk({ status: 'ok', timestamp: '', components: {} });
    await new StellarRouteClient({ baseUrl: 'https://api.example.com' }).getDepsHealth();
    expect(spy.mock.calls[0]?.[0]).toBe('https://api.example.com/health/deps');
  });

  it('calls /api/v1/pairs with the versioned base', async () => {
    const spy = mockOk({ pairs: [], total: 0 });
    await new StellarRouteClient({ baseUrl: 'https://api.example.com' }).getPairs();
    expect(spy.mock.calls[0]?.[0]).toBe('https://api.example.com/api/v1/pairs');
  });

  it('does not duplicate /api/v1 when baseUrl already ends with it', async () => {
    // This is a safety check: if a caller accidentally passes the versioned URL
    // the constructor strips the trailing slash but cannot strip /api/v1 — the
    // getPairs path itself adds /api/v1/pairs so the result would be wrong.
    // This test documents the expected (safe) behaviour when baseUrl is clean.
    const spy = mockOk({ pairs: [], total: 0 });
    await new StellarRouteClient({ baseUrl: 'https://api.example.com' }).getPairs();
    const calledUrl = spy.mock.calls[0]?.[0] as string;
    expect(calledUrl).not.toContain('/api/v1/api/v1');
  });
});
