import { afterEach, describe, expect, it, vi } from 'vitest';

import { batchOrderbook, main, singleOrderbook, USDC, YXLM } from './quickstart-orderbook.js';

function ok(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

const sampleOrderbook = {
  base_asset: { asset_type: 'native' },
  quote_asset: { asset_type: 'credit_alphanum4', asset_code: 'USDC', asset_issuer: 'G...' },
  bids: [{ price: '0.10', amount: '100', total: '10.00' }],
  asks: [{ price: '0.11', amount: '50', total: '5.50' }],
  timestamp: 1_700_000_000,
};

describe('quickstart-orderbook example', () => {
  afterEach(() => vi.restoreAllMocks());

  it('singleOrderbook fetches a single pair and logs its levels', async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(ok(sampleOrderbook));
    vi.stubGlobal('fetch', fetchMock);
    const logSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    await singleOrderbook();

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url] = fetchMock.mock.calls[0] as [string];
    expect(url).toBe(`http://localhost:8080/api/v1/orderbook/native/${encodeURIComponent(USDC)}`);
    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('bids'));
  });

  it('singleOrderbook handles a 404 (no active offers) without throwing', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValueOnce(
        new Response(JSON.stringify({ error: 'not_found', message: 'no offers' }), {
          status: 404,
          headers: { 'Content-Type': 'application/json' },
        }),
      ),
    );
    const logSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    await expect(singleOrderbook()).resolves.toBeUndefined();
    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('no active offers'));
  });

  it('batchOrderbook POSTs both pairs in one request and reports per-item results', async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      ok({
        results: [
          { index: 0, status: 'ok', orderbook: sampleOrderbook },
          {
            index: 1,
            status: 'error',
            error: { code: 'no_route', message: 'No route found' },
          },
        ],
        items_succeeded: 1,
        items_failed: 1,
        total: 2,
      }),
    );
    vi.stubGlobal('fetch', fetchMock);
    const logSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    await batchOrderbook();

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe('http://localhost:8080/api/v1/batch/orderbook');
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body as string)).toEqual({
      requests: [
        { base: 'native', quote: USDC },
        { base: 'native', quote: YXLM },
      ],
    });
    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('1 ok, 1 failed, 2 total'));
  });

  it('main runs both steps in order', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(ok(sampleOrderbook))
      .mockResolvedValueOnce(
        ok({
          results: [],
          items_succeeded: 0,
          items_failed: 0,
          total: 0,
        }),
      );
    vi.stubGlobal('fetch', fetchMock);
    vi.spyOn(console, 'log').mockImplementation(() => {});

    await main();

    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect((fetchMock.mock.calls[0] as [string])[0]).toContain('/api/v1/orderbook/');
    expect((fetchMock.mock.calls[1] as [string])[0]).toContain('/api/v1/batch/orderbook');
  });
});
